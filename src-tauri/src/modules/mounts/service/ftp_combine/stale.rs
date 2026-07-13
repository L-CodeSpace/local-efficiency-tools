/*
 * 核心职责：按 v2 配置路径和工作区 remote 精确清理 FTP combine 残留。
 * 能力边界：不清理用户手动启动或其他配置文件中的 rclone 进程。
 */

use super::super::*;
use super::{super::v2_storage::v2_rclone_config_path, config::workspace_remote_name};

pub(in crate::modules::mounts::service) fn repair_stale_session(
    app: &AppHandle,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    terminate_stale_processes(app, workspace);
    #[cfg(target_os = "macos")]
    cleanup_macos_runtime(app, workspace)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn cleanup_macos_runtime(app: &AppHandle, workspace: &MountWorkspace) -> AppResult<()> {
    let base = super::super::storage::app_rclone_dir(app)?
        .join("workspace-runtime")
        .join(&workspace.id);
    if base.exists() {
        for entry in fs::read_dir(&base)? {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            let target = path.to_string_lossy().to_string();
            let _ = Command::new("/usr/sbin/diskutil")
                .args(["unmount", "force", target.as_str()])
                .output();
            let _ = fs::remove_dir_all(path);
        }
    }
    if let Some(display) = workspace.mount_point.as_deref().map(Path::new) {
        if matches!(fs::read_link(display), Ok(target) if target.starts_with(&base)) {
            let _ = fs::remove_file(display);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn terminate_stale_processes(app: &AppHandle, workspace: &MountWorkspace) {
    let Ok(config) = v2_rclone_config_path(app) else {
        return;
    };
    let config = config.to_string_lossy().replace('\'', "''");
    let remote = format!("{}:", workspace_remote_name(&workspace.id)).replace('\'', "''");
    let script = format!(
        "$config='{}';$remote='{}';Get-CimInstance Win32_Process -Filter \"name='rclone.exe'\" | Where-Object {{ $_.CommandLine -like '* mount *' -and $_.CommandLine -like ('*'+$config+'*') -and $_.CommandLine -like ('*'+$remote+'*') }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        config, remote
    );
    let _ = super::super::normalize::hidden_command(Path::new("powershell"))
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();
}

#[cfg(target_os = "macos")]
fn terminate_stale_processes(app: &AppHandle, workspace: &MountWorkspace) {
    let Ok(config) = v2_rclone_config_path(app) else {
        return;
    };
    let Ok(output) = Command::new("ps").args(["-axo", "pid=,command="]).output() else {
        return;
    };
    let config = config.to_string_lossy();
    let remote = format!("{}:", workspace_remote_name(&workspace.id));
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.contains("rclone")
            && line.contains(" mount ")
            && line.contains(config.as_ref())
            && line.contains(&remote)
        {
            if let Some(pid) = line.split_whitespace().next() {
                let _ = Command::new("kill").args(["-TERM", pid]).status();
            }
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn terminate_stale_processes(_app: &AppHandle, _workspace: &MountWorkspace) {}
