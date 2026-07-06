/*
 * 核心职责：归一化 rclone mount 高级兼容性参数。
 * 业务痛点：VFS 缓存、超时和重试参数需要统一校验，避免无效参数拖垮挂载启动。
 * 能力边界：只处理 profile 内的高级挂载参数，不读取文件或启动进程。
 */

use super::normalize::trim_to_option;
use super::*;

pub(super) fn normalize_cache_mode(value: Option<String>) -> String {
    match trim_to_option(value)
        .unwrap_or_else(|| "full".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => "off".to_string(),
        "minimal" => "minimal".to_string(),
        "writes" => "writes".to_string(),
        _ => "full".to_string(),
    }
}

pub(super) fn normalize_advanced_options(
    input: Option<MountAdvancedOptions>,
    target_is_drive: bool,
) -> AppResult<MountAdvancedOptions> {
    let mut options =
        input.unwrap_or_else(|| recommended_advanced_options_for_target(target_is_drive));
    options.vfs_cache_max_size = normalize_size_suffix(options.vfs_cache_max_size, "VFS 缓存上限")?;
    options.vfs_cache_max_age = normalize_duration(options.vfs_cache_max_age, "VFS 缓存保留时间")?;
    options.vfs_read_chunk_size = normalize_size_suffix(options.vfs_read_chunk_size, "读取块大小")?;
    options.buffer_size = normalize_size_suffix(options.buffer_size, "Buffer 大小")?;
    options.poll_interval = normalize_duration(options.poll_interval, "轮询间隔")?;
    options.connect_timeout = normalize_duration(options.connect_timeout, "连接超时")?;
    options.io_timeout = normalize_duration(options.io_timeout, "IO 超时")?;
    options.retries_sleep = normalize_duration(options.retries_sleep, "重试间隔")?;
    validate_retry_count(options.retries, "重试次数")?;
    validate_retry_count(options.low_level_retries, "低层重试次数")?;
    if !target_is_drive {
        options.network_mode = false;
    }
    Ok(options)
}

pub(super) fn recommended_advanced_options_for_target(
    target_is_drive: bool,
) -> MountAdvancedOptions {
    let mut options = MountAdvancedOptions::default();
    if !target_is_drive {
        options.network_mode = false;
    }
    options
}

pub(super) fn normalize_size_suffix(value: String, label: &str) -> AppResult<String> {
    let value = value.trim().to_string();
    if value.eq_ignore_ascii_case("off") {
        return Ok("off".to_string());
    }
    if value.is_empty() || !is_rclone_size_suffix(&value) {
        return Err(AppError::new(
            "mount_invalid_advanced_option",
            format!("{}格式无效，请使用 32M、5G 或 off", label),
        ));
    }
    Ok(value)
}

pub(super) fn normalize_duration(value: String, label: &str) -> AppResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() || !is_rclone_duration(&value) {
        return Err(AppError::new(
            "mount_invalid_advanced_option",
            format!("{}格式无效，请使用 0、5s、30s 或 24h", label),
        ));
    }
    Ok(value)
}

pub(super) fn validate_retry_count(value: u16, label: &str) -> AppResult<()> {
    if value > 100 {
        return Err(AppError::new(
            "mount_invalid_advanced_option",
            format!("{}不能超过 100", label),
        ));
    }
    Ok(())
}

pub(super) fn is_rclone_size_suffix(value: &str) -> bool {
    let mut chars = value.chars().peekable();
    let mut has_digit = false;
    while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
        has_digit = true;
        chars.next();
    }
    has_digit && chars.all(|ch| ch.is_ascii_alphabetic())
}

pub(super) fn is_rclone_duration(value: &str) -> bool {
    if value == "0" {
        return true;
    }
    let bytes = value.as_bytes();
    let mut index = 0;
    let mut segments = 0;
    while index < bytes.len() {
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if start == index || index >= bytes.len() {
            return false;
        }
        if index + 1 < bytes.len() && &value[index..index + 2] == "ms" {
            index += 2;
        } else if matches!(bytes[index], b's' | b'm' | b'h') {
            index += 1;
        } else {
            return false;
        }
        segments += 1;
    }
    segments > 0
}
