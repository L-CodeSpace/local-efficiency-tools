/*
 * 核心职责：转换前端挂载表单。
 * 业务痛点：表单输入到持久化 profile 的映射需要单独校验。
 * 能力边界：只处理输入归一化和 profile 构建。
 */

use super::*;
use super::{
    advanced_options::{normalize_advanced_options, normalize_cache_mode},
    normalize::{
        blank, normalize_input_drive_letter, normalize_tls_mode, now_millis, trim_to_option,
    },
    storage::default_mount_target,
};

pub(super) fn build_profile_from_input(
    app: &AppHandle,
    input: MountProfileInput,
    existing: Option<&MountProfile>,
) -> AppResult<MountProfile> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::new("mount_invalid_name", "配置名称不能为空"));
    }

    if matches!(input.protocol, MountProtocol::Ftp | MountProtocol::Sftp) && blank(&input.host) {
        return Err(AppError::new(
            "mount_invalid_host",
            "FTP/SFTP 必须填写主机地址",
        ));
    }
    if matches!(input.protocol, MountProtocol::Webdav) && blank(&input.url) {
        return Err(AppError::new(
            "mount_invalid_url",
            "WebDAV 必须填写服务 URL",
        ));
    }

    let now = now_millis();
    let id = existing
        .map(|profile| profile.id.clone())
        .or(input.id)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let remote_name = existing
        .map(|profile| profile.remote_name.clone())
        .unwrap_or_else(|| format!("local_efficiency_{}", id.replace('-', "")));

    let mut mount_point = trim_to_option(input.mount_point);
    let drive_letter = normalize_input_drive_letter(input.drive_letter);
    if mount_point.is_none() && drive_letter.is_none() {
        let default_target = default_mount_target(app, &name)?;
        mount_point = Some(default_target.to_string_lossy().to_string());
    }

    let protocol = input.protocol;
    let tls_mode = normalize_tls_mode(&protocol, input.tls_mode);
    let advanced_options =
        normalize_advanced_options(input.advanced_options, drive_letter.is_some())?;

    Ok(MountProfile {
        id,
        name,
        protocol,
        remote_name,
        host: trim_to_option(input.host),
        port: input.port,
        username: trim_to_option(input.username),
        password: existing.and_then(|profile| profile.password.clone()),
        url: trim_to_option(input.url),
        vendor: trim_to_option(input.vendor),
        key_file: trim_to_option(input.key_file),
        remote_path: trim_to_option(input.remote_path),
        mount_point,
        drive_letter,
        tls_mode,
        no_check_certificate: input.no_check_certificate.unwrap_or(false),
        read_only: input.read_only.unwrap_or(false),
        cache_mode: normalize_cache_mode(input.cache_mode),
        advanced_options,
        enabled: input
            .enabled
            .unwrap_or_else(|| existing.map(|profile| profile.enabled).unwrap_or(false)),
        created_at: existing.map(|profile| profile.created_at).unwrap_or(now),
        updated_at: now,
        mounted: false,
        status: MountStatus::Disabled,
        error: None,
    })
}
