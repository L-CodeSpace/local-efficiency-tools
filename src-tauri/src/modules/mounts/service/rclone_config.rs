/*
 * 核心职责：同步 rclone remote 配置。
 * 业务痛点：remote 配置命令参数必须集中生成，避免协议差异散落。
 * 能力边界：只处理 rclone config 子命令参数。
 */

use super::profiles::MountPasswordUpdate;
use super::*;
use super::{
    normalize::{
        hidden_command, is_drive_target, push_opt, read_remote_option, remote_exists,
        remove_blank_remote_option, remove_remote_option,
    },
    runtime_download::ensure_rclone,
    storage::rclone_config_path,
    target::mount_target,
};

pub(super) fn sync_rclone_config(
    app: &AppHandle,
    profile: &MountProfile,
    password_update: &MountPasswordUpdate,
) -> AppResult<()> {
    let rclone_path = ensure_rclone(app)?;
    let config_path = rclone_config_path(app)?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut options = config_options(profile);
    if let MountPasswordUpdate::Set(password) = password_update {
        options.push(("pass".to_string(), password.clone()));
    }
    if options.is_empty() {
        if matches!(password_update, MountPasswordUpdate::Clear) {
            remove_remote_option(&config_path, &profile.remote_name, "pass")?;
        }
        return Ok(());
    }

    let action = if remote_exists(&config_path, &profile.remote_name)? {
        "update"
    } else {
        "create"
    };

    let result = match run_config_action(&rclone_path, &config_path, action, profile, &options) {
        Ok(()) => Ok(()),
        Err(error) if action == "update" => {
            observability::emit_info(
                app,
                format!("rclone config update 失败，改用 create: {}", error),
            );
            run_config_action(&rclone_path, &config_path, "create", profile, &options)
        }
        Err(error) => Err(error),
    };

    result?;
    match password_update {
        MountPasswordUpdate::Clear => {
            remove_remote_option(&config_path, &profile.remote_name, "pass")?;
        }
        MountPasswordUpdate::Unchanged => {
            remove_blank_remote_option(&config_path, &profile.remote_name, "pass")?;
        }
        MountPasswordUpdate::Set(_) => {}
    }
    validate_remote_password_config(&rclone_path, &config_path, &profile.remote_name)
}

pub(super) fn run_config_action(
    rclone_path: &Path,
    config_path: &Path,
    action: &str,
    profile: &MountProfile,
    options: &[(String, String)],
) -> AppResult<()> {
    let mut command = hidden_command(rclone_path);
    command.args(build_config_args(config_path, action, profile, options));

    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(AppError::new(
            "mount_config_failed",
            if stderr.is_empty() { stdout } else { stderr },
        ))
    }
}

pub(super) fn build_config_args(
    config_path: &Path,
    action: &str,
    profile: &MountProfile,
    options: &[(String, String)],
) -> Vec<String> {
    let mut args = vec![
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
        "config".to_string(),
        "--obscure".to_string(),
        "--non-interactive".to_string(),
        action.to_string(),
        profile.remote_name.clone(),
        profile.protocol.as_str().to_string(),
    ];

    for (key, value) in options {
        args.push(key.clone());
        args.push(value.clone());
    }
    args
}

pub(super) fn config_options(profile: &MountProfile) -> Vec<(String, String)> {
    let mut options = Vec::new();
    match profile.protocol {
        MountProtocol::Ftp => {
            push_opt(&mut options, "host", profile.host.as_deref());
            push_opt(&mut options, "user", profile.username.as_deref());
            if let Some(port) = profile.port {
                options.push(("port".to_string(), port.to_string()));
            }
            match profile.tls_mode.as_deref() {
                Some("explicit") => options.push(("explicit_tls".to_string(), "true".to_string())),
                Some("implicit") => options.push(("tls".to_string(), "true".to_string())),
                _ => {}
            }
        }
        MountProtocol::Sftp => {
            push_opt(&mut options, "host", profile.host.as_deref());
            push_opt(&mut options, "user", profile.username.as_deref());
            push_opt(&mut options, "key_file", profile.key_file.as_deref());
            if let Some(port) = profile.port {
                options.push(("port".to_string(), port.to_string()));
            }
        }
        MountProtocol::Webdav => {
            push_opt(&mut options, "url", profile.url.as_deref());
            push_opt(&mut options, "vendor", profile.vendor.as_deref());
            push_opt(&mut options, "user", profile.username.as_deref());
        }
    }
    options
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_mount_args(
    config_path: &Path,
    cache_dir: &Path,
    profile: &MountProfile,
) -> AppResult<Vec<String>> {
    let target = mount_target(profile)?;
    build_mount_args_for_target(config_path, cache_dir, profile, &target)
}

pub(super) fn build_mount_args_for_target(
    config_path: &Path,
    cache_dir: &Path,
    profile: &MountProfile,
    target: &Path,
) -> AppResult<Vec<String>> {
    let advanced = &profile.advanced_options;
    let mut args = vec![
        "--config".to_string(),
        config_path.to_string_lossy().to_string(),
        "mount".to_string(),
        remote_source(profile),
        target.to_string_lossy().to_string(),
        "--cache-dir".to_string(),
        cache_dir.to_string_lossy().to_string(),
        "--vfs-cache-mode".to_string(),
        profile.cache_mode.clone(),
        "--vfs-cache-max-size".to_string(),
        advanced.vfs_cache_max_size.clone(),
        "--vfs-cache-max-age".to_string(),
        advanced.vfs_cache_max_age.clone(),
        "--vfs-read-chunk-size".to_string(),
        advanced.vfs_read_chunk_size.clone(),
        "--buffer-size".to_string(),
        advanced.buffer_size.clone(),
        "--poll-interval".to_string(),
        advanced.poll_interval.clone(),
        "--contimeout".to_string(),
        advanced.connect_timeout.clone(),
        "--timeout".to_string(),
        advanced.io_timeout.clone(),
        "--retries".to_string(),
        advanced.retries.to_string(),
        "--low-level-retries".to_string(),
        advanced.low_level_retries.to_string(),
        "--retries-sleep".to_string(),
        advanced.retries_sleep.clone(),
        "--log-level".to_string(),
        "INFO".to_string(),
    ];
    if advanced.links {
        args.push("--links".to_string());
    }
    if cfg!(target_os = "windows") && is_drive_target(&target) && advanced.network_mode {
        args.push("--network-mode".to_string());
    }
    if profile.read_only {
        args.push("--read-only".to_string());
    }
    if profile.no_check_certificate {
        args.push("--no-check-certificate".to_string());
    }
    Ok(args)
}

pub(super) fn remote_source(profile: &MountProfile) -> String {
    let remote_path = profile
        .remote_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .trim_start_matches('/');
    if remote_path.is_empty() {
        format!("{}:", profile.remote_name)
    } else {
        format!("{}:{}", profile.remote_name, remote_path)
    }
}

pub(super) fn validate_remote_password_config(
    rclone_path: &Path,
    config_path: &Path,
    remote_name: &str,
) -> AppResult<()> {
    let Some(password) = read_remote_option(config_path, remote_name, "pass")? else {
        return Ok(());
    };
    if password.trim().is_empty() {
        remove_remote_option(config_path, remote_name, "pass")?;
        return Ok(());
    }

    let output = hidden_command(rclone_path)
        .arg("reveal")
        .arg(password.trim())
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    remove_remote_option(config_path, remote_name, "pass")?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    let error = AppError::new(
        "mount_password_invalid",
        "rclone 密码配置已损坏，请编辑挂载配置并重新输入密码后再启用。",
    );
    if detail.is_empty() {
        Err(error)
    } else {
        Err(error.with_detail(detail))
    }
}

pub(super) fn reveal_obscured_password(
    rclone_path: &Path,
    obscured_password: &str,
) -> AppResult<Option<String>> {
    let obscured_password = obscured_password.trim();
    if obscured_password.is_empty() {
        return Ok(None);
    }

    let output = hidden_command(rclone_path)
        .arg("reveal")
        .arg(obscured_password)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if password.is_empty() {
        Ok(None)
    } else {
        Ok(Some(password))
    }
}
