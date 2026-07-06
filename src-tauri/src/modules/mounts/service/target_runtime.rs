/*
 * 核心职责：解析 rclone 实际挂载目标并分发平台清理策略。
 * 业务痛点：macOS 需要内部运行时目录，Windows 需要网络盘符清理。
 * 能力边界：只提供进程模块使用的挂载目标 API。
 */

#[cfg(windows)]
use super::normalize::hidden_command;
#[cfg(windows)]
use super::normalize::is_drive_target;
use super::target::prepare_mount_target;
use super::*;

#[cfg(target_os = "macos")]
#[path = "target_runtime/macos.rs"]
mod macos;

pub(super) struct EffectiveMountTarget {
    pub actual: PathBuf,
    pub display: Option<PathBuf>,
}

#[cfg(target_os = "macos")]
pub(super) fn prepare_effective_mount_target(
    app: &AppHandle,
    profile: &MountProfile,
    display_target: &Path,
    network_mode: bool,
) -> AppResult<EffectiveMountTarget> {
    macos::prepare_effective_mount_target(app, profile, display_target, network_mode)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn prepare_effective_mount_target(
    app: &AppHandle,
    profile: &MountProfile,
    display_target: &Path,
    network_mode: bool,
) -> AppResult<EffectiveMountTarget> {
    cleanup_mount_target(app, display_target, network_mode, &profile.name);
    prepare_mount_target(display_target)?;
    Ok(EffectiveMountTarget {
        actual: display_target.to_path_buf(),
        display: None,
    })
}

#[cfg(target_os = "macos")]
pub(super) fn create_display_symlink(display_target: &Path, actual_target: &Path) -> AppResult<()> {
    macos::create_display_symlink(display_target, actual_target)
}

#[cfg(target_os = "macos")]
pub(super) fn cleanup_display_target(
    app: &AppHandle,
    display_target: Option<&Path>,
    actual_target: &Path,
) {
    macos::cleanup_display_target(app, display_target, actual_target);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn cleanup_display_target(
    _app: &AppHandle,
    _display_target: Option<&Path>,
    _actual_target: &Path,
) {
}

#[cfg(windows)]
pub(super) fn cleanup_mount_target(
    app: &AppHandle,
    target: &Path,
    network_mode: bool,
    profile_name: &str,
) {
    if !network_mode || !is_drive_target(target) {
        return;
    }

    let drive = target.to_string_lossy().trim().to_string();
    if drive.is_empty() {
        return;
    }

    let output = hidden_command(Path::new("net"))
        .args(["use", drive.as_str(), "/delete", "/y"])
        .output();
    if matches!(output, Ok(output) if output.status.success()) {
        observability::emit_info(
            app,
            format!("已清理 rclone 网络盘符: {} ({})", drive, profile_name),
        );
    }
}

#[cfg(target_os = "macos")]
pub(super) fn cleanup_mount_target(
    app: &AppHandle,
    target: &Path,
    network_mode: bool,
    profile_name: &str,
) {
    macos::cleanup_mount_target(app, target, network_mode, profile_name);
}

#[cfg(all(not(windows), not(target_os = "macos")))]
pub(super) fn cleanup_mount_target(
    _app: &AppHandle,
    _target: &Path,
    _network_mode: bool,
    _profile_name: &str,
) {
}
