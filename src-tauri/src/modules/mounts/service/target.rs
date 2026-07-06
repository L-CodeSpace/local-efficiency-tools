/*
 * 核心职责：准备本地挂载目标。
 * 业务痛点：挂载点处理存在平台差异，必须独立防御。
 * 能力边界：只处理目录、盘符和目标冲突。
 */

use super::normalize::{is_drive_target, now_millis};
use super::*;

pub(super) fn mount_target(profile: &MountProfile) -> AppResult<PathBuf> {
    if let Some(letter) = profile
        .drive_letter
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(PathBuf::from(letter));
    }
    if let Some(path) = profile
        .mount_point
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    Err(AppError::new(
        "mount_target_missing",
        "缺少本地挂载点或盘符",
    ))
}

pub(super) fn prepare_mount_target(target: &Path) -> AppResult<()> {
    if is_drive_target(target) {
        return Ok(());
    }

    prepare_directory_mount_target(target)
}

#[cfg(windows)]
pub(super) fn prepare_directory_mount_target(target: &Path) -> AppResult<()> {
    if target.exists() {
        if target.is_dir() && fs::read_dir(target)?.next().is_none() {
            // WinFsp 要求目录挂载点不存在；空目录可安全移除，非空目录必须保护用户数据。
            fs::remove_dir(target)?;
        } else {
            return Err(mount_target_exists_error(target));
        }
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn prepare_directory_mount_target(target: &Path) -> AppResult<()> {
    fs::create_dir_all(target)?;
    Ok(())
}

#[cfg(windows)]
pub(super) fn mount_target_exists_error(target: &Path) -> AppError {
    AppError::new("mount_target_exists", "挂载路径已存在，请选择处理方式").with_detail(
        serde_json::json!({
            "target": target.to_string_lossy(),
            "suggested": suggested_mount_target(target).to_string_lossy(),
        })
        .to_string(),
    )
}

#[cfg(windows)]
pub(super) fn suggested_mount_target(target: &Path) -> PathBuf {
    let parent = target.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("mount");

    for index in 1..1000 {
        let candidate = parent.join(format!("{name}-{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{name}-{}", now_millis()))
}
