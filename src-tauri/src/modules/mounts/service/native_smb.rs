/*
 * 核心职责：编排原生 SMB 工作区的挂载、回滚、卸载和修复。
 * 能力边界：平台 API 分别由 Windows WNet 与 macOS NetFS 适配器实现。
 */

use super::*;
use crate::modules::state::NativeSmbMount;

#[path = "native_smb/common.rs"]
mod common;
#[cfg(target_os = "macos")]
#[path = "native_smb/macos.rs"]
mod macos;
#[cfg(windows)]
#[path = "native_smb/windows.rs"]
mod windows;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use windows as platform;

pub(super) fn mount_workspace(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<Vec<NativeSmbMount>> {
    let mut mounted = Vec::new();
    for binding in &workspace.bindings {
        match mount_binding(app, connection, workspace, binding) {
            Ok(item) => mounted.push(item),
            Err(error) => {
                for item in mounted.iter().rev() {
                    unmount_item(app, item);
                }
                return Err(error);
            }
        }
    }
    Ok(mounted)
}

pub(super) fn unmount_workspace(app: &AppHandle, mounts: &[NativeSmbMount]) {
    for mount in mounts.iter().rev() {
        unmount_item(app, mount);
    }
}

pub(super) fn repair_workspace(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    repair_workspace_impl(app, connection, workspace)
}

#[cfg(any(windows, target_os = "macos"))]
fn mount_binding(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
    binding: &RemoteBinding,
) -> AppResult<NativeSmbMount> {
    platform::mount_binding(app, connection, workspace, binding)
}

#[cfg(any(windows, target_os = "macos"))]
fn unmount_item(app: &AppHandle, mount: &NativeSmbMount) {
    platform::unmount_item(app, mount);
}

#[cfg(any(windows, target_os = "macos"))]
fn repair_workspace_impl(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    platform::repair_workspace(app, connection, workspace)
}

#[cfg(not(any(windows, target_os = "macos")))]
fn mount_binding(
    _app: &AppHandle,
    _connection: &RemoteConnection,
    _workspace: &MountWorkspace,
    _binding: &RemoteBinding,
) -> AppResult<NativeSmbMount> {
    Err(AppError::new(
        "mount_smb_unsupported",
        "当前平台不支持原生 SMB 挂载",
    ))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn unmount_item(_app: &AppHandle, _mount: &NativeSmbMount) {}

#[cfg(not(any(windows, target_os = "macos")))]
fn repair_workspace_impl(
    _app: &AppHandle,
    _connection: &RemoteConnection,
    _workspace: &MountWorkspace,
) -> AppResult<()> {
    Ok(())
}
