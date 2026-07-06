/*
 * 核心职责：管理挂载配置和后台设置。
 * 业务痛点：配置增删改与后台运行设置需要保留稳定 API。
 * 能力边界：只处理对外 profile 操作和设置读写入口。
 */

use super::*;
use super::{
    advanced_options::{normalize_advanced_options, recommended_advanced_options_for_target},
    normalize::{delete_remote_from_config, hidden_command, now_millis, read_remote_option},
    processes::{hydrate_runtime_status, start_mount, stop_all, stop_mount},
    profile_form::build_profile_from_input,
    rclone_config::{remote_source, reveal_obscured_password, sync_rclone_config},
    runtime_download::ensure_rclone,
    storage::{
        background_settings_path, load_profiles, rclone_binary_path, rclone_config_path,
        read_background_settings, replace_profile, save_profiles,
    },
};

pub fn list_profiles(app: &AppHandle, state: &AppState) -> AppResult<Vec<MountProfile>> {
    let mut profiles = load_profiles(app)?;
    let mut changed = upgrade_profile_defaults(&mut profiles);
    changed |= backfill_plaintext_passwords(app, &mut profiles)?;
    if changed {
        save_profiles(app, &profiles)?;
    }
    for profile in profiles.iter_mut() {
        hydrate_runtime_status(state, profile);
    }
    Ok(profiles)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MountPasswordUpdate {
    Unchanged,
    Set(String),
    Clear,
}

pub(super) fn password_update_from_input(password: Option<&str>) -> MountPasswordUpdate {
    match password {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                MountPasswordUpdate::Clear
            } else {
                MountPasswordUpdate::Set(trimmed.to_string())
            }
        }
        None => MountPasswordUpdate::Unchanged,
    }
}

pub fn save_profile(
    app: AppHandle,
    state: AppState,
    input: MountProfileInput,
) -> AppResult<MountProfile> {
    let password_update = password_update_from_input(input.password.as_deref());
    let mut profiles = load_profiles(&app)?;
    let existing_index = input
        .id
        .as_ref()
        .and_then(|id| profiles.iter().position(|profile| &profile.id == id));
    let existing = existing_index.map(|index| profiles[index].clone());
    let mut profile = build_profile_from_input(&app, input, existing.as_ref())?;
    profile.password = match &password_update {
        MountPasswordUpdate::Unchanged => existing
            .as_ref()
            .and_then(|profile| profile.password.clone()),
        MountPasswordUpdate::Set(password) => Some(password.clone()),
        MountPasswordUpdate::Clear => None,
    };

    sync_rclone_config(&app, &profile, &password_update)?;

    if let Some(index) = existing_index {
        profiles[index] = profile.clone();
    } else {
        profiles.push(profile.clone());
    }
    save_profiles(&app, &profiles)?;

    if profile.enabled {
        if let Err(error) = start_mount(&app, &state, &profile) {
            profile.enabled = false;
            profile.updated_at = now_millis();
            replace_profile(&app, profile.clone())?;
            return Err(error);
        }
    } else {
        stop_mount(&app, &state, &profile.id)?;
    }

    hydrate_runtime_status(&state, &mut profile);
    Ok(profile)
}

pub fn delete_profile(app: AppHandle, state: AppState, id: String) -> AppResult<()> {
    stop_mount(&app, &state, &id)?;
    let mut profiles = load_profiles(&app)?;
    let Some(index) = profiles.iter().position(|profile| profile.id == id) else {
        return Err(AppError::new("mount_profile_not_found", "未找到挂载配置"));
    };
    let profile = profiles.remove(index);
    save_profiles(&app, &profiles)?;
    delete_remote_from_config(&rclone_config_path(&app)?, &profile.remote_name)
}

pub fn set_profile_enabled(
    app: AppHandle,
    state: AppState,
    id: String,
    enabled: bool,
) -> AppResult<MountProfile> {
    let mut profiles = load_profiles(&app)?;
    if upgrade_profile_defaults(&mut profiles) {
        save_profiles(&app, &profiles)?;
    }
    let index = profiles
        .iter()
        .position(|profile| profile.id == id)
        .ok_or_else(|| AppError::new("mount_profile_not_found", "未找到挂载配置"))?;

    if enabled {
        profiles[index].enabled = true;
        profiles[index].updated_at = now_millis();
        save_profiles(&app, &profiles)?;
        if let Err(error) = start_mount(&app, &state, &profiles[index]) {
            profiles[index].enabled = false;
            profiles[index].updated_at = now_millis();
            save_profiles(&app, &profiles)?;
            return Err(error);
        }
    } else {
        stop_mount(&app, &state, &id)?;
        profiles[index].enabled = false;
        profiles[index].updated_at = now_millis();
        save_profiles(&app, &profiles)?;
    }

    let mut profile = profiles[index].clone();
    hydrate_runtime_status(&state, &mut profile);
    Ok(profile)
}

pub fn test_profile(app: AppHandle, id: String) -> AppResult<MountTestResult> {
    let mut profiles = load_profiles(&app)?;
    if upgrade_profile_defaults(&mut profiles) {
        save_profiles(&app, &profiles)?;
    }
    let profile = profiles
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| AppError::new("mount_profile_not_found", "未找到挂载配置"))?;
    let rclone_path = ensure_rclone(&app)?;
    let output = hidden_command(&rclone_path)
        .arg("--config")
        .arg(rclone_config_path(&app)?)
        .arg("lsf")
        .arg(remote_source(profile))
        .arg("--max-depth")
        .arg("1")
        .arg("--contimeout")
        .arg("10s")
        .arg("--timeout")
        .arg("20s")
        .output()?;

    if output.status.success() {
        Ok(MountTestResult {
            success: true,
            message: "连接测试成功。".to_string(),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(MountTestResult {
            success: false,
            message: if stderr.is_empty() { stdout } else { stderr },
        })
    }
}

pub fn unmount_all(app: AppHandle, state: AppState) -> AppResult<()> {
    stop_all(&app, &state);
    let mut profiles = load_profiles(&app)?;
    for profile in profiles.iter_mut() {
        profile.enabled = false;
        profile.updated_at = now_millis();
    }
    save_profiles(&app, &profiles)
}

pub fn get_background_settings(app: &AppHandle) -> AppResult<BackgroundSettings> {
    read_background_settings(app)
}

pub fn set_background_enabled(app: &AppHandle, enabled: bool) -> AppResult<BackgroundSettings> {
    let settings = BackgroundSettings { enabled };
    let path = background_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(&settings).map_err(|error| {
        AppError::new("mount_settings_serialize_failed", "序列化后台运行设置失败")
            .with_detail(error.to_string())
    })?;
    fs::write(path, content)?;
    Ok(settings)
}

pub(super) fn backfill_plaintext_passwords(
    app: &AppHandle,
    profiles: &mut [MountProfile],
) -> AppResult<bool> {
    let rclone_path = rclone_binary_path(app)?;
    let config_path = rclone_config_path(app)?;
    if !rclone_path.exists() || !config_path.exists() {
        return Ok(false);
    }

    let mut changed = false;
    for profile in profiles.iter_mut() {
        if profile.password.is_some() {
            continue;
        }
        let Some(obscured_password) =
            read_remote_option(&config_path, &profile.remote_name, "pass")?
        else {
            continue;
        };
        let password = match reveal_obscured_password(&rclone_path, &obscured_password) {
            Ok(Some(password)) => password,
            Ok(None) => continue,
            Err(_error) => {
                // 【合理吞噬】旧配置明文密码回填只是编辑体验兼容增强，失败时必须保留原 profile 与 rclone 配置继续可用。
                continue;
            }
        };
        profile.password = Some(password);
        changed = true;
    }
    Ok(changed)
}

pub(super) fn upgrade_profile_defaults(profiles: &mut [MountProfile]) -> bool {
    let mut changed = false;
    for profile in profiles.iter_mut() {
        if profile.protocol.as_str() == "ftp" && profile.cache_mode.eq_ignore_ascii_case("writes") {
            profile.cache_mode = "full".to_string();
            changed = true;
        }

        let target_is_drive = profile
            .drive_letter
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some();
        let next_options =
            normalize_advanced_options(Some(profile.advanced_options.clone()), target_is_drive)
                .unwrap_or_else(|_| recommended_advanced_options_for_target(target_is_drive));
        if profile.advanced_options != next_options {
            profile.advanced_options = next_options;
            changed = true;
        }
    }
    changed
}
