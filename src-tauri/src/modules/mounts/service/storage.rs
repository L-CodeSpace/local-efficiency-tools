/*
 * 核心职责：读写挂载持久化文件。
 * 业务痛点：配置、日志和缓存路径必须集中维护。
 * 能力边界：只处理本应用数据目录下的文件路径和 JSON 存储。
 */

use super::normalize::sanitize_path_part;
use super::*;

pub(super) fn load_profiles(app: &AppHandle) -> AppResult<Vec<MountProfile>> {
    let path = profiles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| {
        AppError::new("mount_profiles_parse_failed", "解析 rclone 配置失败")
            .with_detail(error.to_string())
    })
}

pub(super) fn save_profiles(app: &AppHandle, profiles: &[MountProfile]) -> AppResult<()> {
    let path = profiles_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(profiles).map_err(|error| {
        AppError::new("mount_profiles_serialize_failed", "序列化 rclone 配置失败")
            .with_detail(error.to_string())
    })?;
    fs::write(path, content)?;
    Ok(())
}

pub(super) fn replace_profile(app: &AppHandle, profile: MountProfile) -> AppResult<()> {
    let mut profiles = load_profiles(app)?;
    if let Some(index) = profiles
        .iter()
        .position(|existing| existing.id == profile.id)
    {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
    save_profiles(app, &profiles)
}

pub(super) fn read_background_settings(app: &AppHandle) -> AppResult<BackgroundSettings> {
    let path = background_settings_path(app)?;
    if !path.exists() {
        return Ok(BackgroundSettings::default());
    }
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| {
        AppError::new("mount_settings_parse_failed", "解析后台运行设置失败")
            .with_detail(error.to_string())
    })
}

pub(super) fn app_rclone_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?.join("rclone");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn rclone_runtime_dir(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("runtime"))
}

pub(super) fn rclone_binary_path(app: &AppHandle) -> AppResult<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        "rclone.exe"
    } else {
        "rclone"
    };
    Ok(rclone_runtime_dir(app)?.join(name))
}

pub(super) fn rclone_config_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("rclone.conf"))
}

pub(super) fn profiles_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("profiles.json"))
}

pub(super) fn background_settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("background.json"))
}

pub(super) fn profile_cache_dir(app: &AppHandle, id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("cache").join(id))
}

pub(super) fn profile_log_path(app: &AppHandle, id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?
        .join("logs")
        .join(format!("{}.log", id)))
}

pub(super) fn mounts_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app_rclone_dir(app)?.join("mounts");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn default_mount_target(app: &AppHandle, name: &str) -> AppResult<PathBuf> {
    let (root, _) = default_mount_root(app)?;
    Ok(root.join(default_mount_dir_name(name)))
}

pub(super) fn default_drive_letter(app: &AppHandle) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let profiles = load_profiles(app).unwrap_or_default();
        let used = profiles
            .iter()
            .filter_map(|profile| profile.drive_letter.as_deref())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        select_default_drive_letter(&used, |letter| {
            PathBuf::from(format!("{}:\\", letter)).exists()
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        None
    }
}

#[cfg(target_os = "windows")]
pub(super) fn select_default_drive_letter(
    used_letters: &[String],
    is_occupied: impl Fn(char) -> bool,
) -> Option<String> {
    for code in (b'D'..=b'Z').rev() {
        let letter = code as char;
        if used_letters
            .iter()
            .filter_map(|value| value.chars().next())
            .any(|value| value.eq_ignore_ascii_case(&letter))
        {
            continue;
        }
        if is_occupied(letter) {
            continue;
        }
        return Some(format!("{}:", letter));
    }
    None
}

pub(super) fn default_mount_root(app: &AppHandle) -> AppResult<(PathBuf, bool)> {
    match app.path().desktop_dir() {
        Ok(desktop) => Ok((desktop, true)),
        Err(_) => Ok((mounts_dir(app)?, false)),
    }
}

pub(super) fn default_mount_dir_name(name: &str) -> String {
    sanitize_path_part(name)
}
