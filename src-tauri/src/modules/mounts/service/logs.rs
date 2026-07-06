/*
 * 核心职责：读取 rclone profile 日志。
 * 业务痛点：挂载失败和资源管理器卡顿需要可追溯的 rclone 原始输出。
 * 能力边界：只允许按已保存 profile 读取固定日志文件，不接受任意路径。
 */

use super::storage::{load_profiles, profile_log_path};
use super::*;

pub(super) const DEFAULT_LOG_MAX_LINES: usize = 400;
pub(super) const MAX_LOG_MAX_LINES: usize = 2000;

pub fn get_profile_log(
    app: AppHandle,
    id: String,
    max_lines: Option<usize>,
) -> AppResult<MountProfileLog> {
    let profiles = load_profiles(&app)?;
    let profile = profiles
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| AppError::new("mount_profile_not_found", "未找到挂载配置"))?;
    let path = profile_log_path(&app, &profile.id)?;
    let path_text = path.to_string_lossy().to_string();

    if !path.exists() {
        return Ok(MountProfileLog {
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            path: path_text,
            exists: false,
            size_bytes: 0,
            modified_at: None,
            content: String::new(),
        });
    }

    let metadata = fs::metadata(&path)?;
    if !metadata.is_file() {
        return Err(AppError::new(
            "mount_log_invalid",
            "rclone 日志路径不是普通文件",
        ));
    }

    Ok(MountProfileLog {
        profile_id: profile.id.clone(),
        profile_name: profile.name.clone(),
        path: path_text,
        exists: true,
        size_bytes: metadata.len(),
        modified_at: metadata.modified().ok().and_then(system_time_millis),
        content: read_log_tail(&path, max_lines.unwrap_or(DEFAULT_LOG_MAX_LINES))?,
    })
}

pub(super) fn read_log_tail(path: &Path, max_lines: usize) -> AppResult<String> {
    let content = fs::read_to_string(path)?;
    Ok(tail_log_content(
        &content,
        normalize_log_line_limit(max_lines),
    ))
}

pub(super) fn normalize_log_line_limit(max_lines: usize) -> usize {
    max_lines.clamp(1, MAX_LOG_MAX_LINES)
}

pub(super) fn tail_log_content(content: &str, max_lines: usize) -> String {
    let mut lines = content
        .lines()
        .rev()
        .take(normalize_log_line_limit(max_lines))
        .collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
}

pub(super) fn system_time_millis(value: SystemTime) -> Option<u64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}
