/*
 * 核心职责：探测平台挂载依赖。
 * 业务痛点：WinFsp 和 macFUSE 检测逻辑有平台差异。
 * 能力边界：只处理系统依赖探测。
 */

use super::normalize::hidden_command;
use super::*;

pub(super) fn current_platform() -> MountPlatform {
    if cfg!(target_os = "windows") {
        MountPlatform::Windows
    } else if cfg!(target_os = "macos") {
        MountPlatform::Macos
    } else if cfg!(target_os = "linux") {
        MountPlatform::Linux
    } else {
        MountPlatform::Unknown
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy)]
pub(super) struct WindowsFuseProbe {
    pub(super) winfsp: bool,
    pub(super) sshfs_win: bool,
}

#[cfg(target_os = "windows")]
pub(super) fn probe_windows_fuse() -> WindowsFuseProbe {
    WindowsFuseProbe {
        winfsp: has_winfsp(),
        sshfs_win: has_sshfs_win(),
    }
}

#[cfg(target_os = "windows")]
pub(super) fn has_winfsp() -> bool {
    has_any_path(&[
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files (x86)\WinFsp\bin\winfsp-x86.dll",
        r"C:\Program Files (x86)\WinFsp\bin\fsptool-x64.exe",
        r"C:\Program Files\WinFsp\bin\winfsp-x64.dll",
        r"C:\Program Files\WinFsp\bin\winfsp-x86.dll",
        r"C:\Program Files\WinFsp\bin\fsptool-x64.exe",
    ]) || registry_key_exists(r"HKLM\SOFTWARE\WinFsp")
        || registry_key_exists(r"HKLM\SOFTWARE\WOW6432Node\WinFsp")
        || registry_uninstall_contains("WinFsp")
        || windows_service_exists("WinFsp.Launcher")
}

#[cfg(target_os = "windows")]
pub(super) fn has_sshfs_win() -> bool {
    has_any_path(&[
        r"C:\Program Files\SSHFS-Win\bin\sshfs-win.exe",
        r"C:\Program Files (x86)\SSHFS-Win\bin\sshfs-win.exe",
    ]) || registry_uninstall_contains("SSHFS-Win")
}

#[cfg(target_os = "windows")]
pub(super) fn has_any_path(paths: &[&str]) -> bool {
    paths.iter().any(|path| Path::new(path).exists())
}

#[cfg(target_os = "windows")]
pub(super) fn registry_key_exists(key: &str) -> bool {
    hidden_command(Path::new("reg.exe"))
        .arg("query")
        .arg(key)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
pub(super) fn registry_uninstall_contains(needle: &str) -> bool {
    [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ]
    .iter()
    .any(|root| registry_tree_contains(root, needle))
}

#[cfg(target_os = "windows")]
pub(super) fn registry_tree_contains(root: &str, needle: &str) -> bool {
    let Ok(output) = hidden_command(Path::new("reg.exe"))
        .arg("query")
        .arg(root)
        .arg("/s")
        .arg("/f")
        .arg(needle)
        .arg("/d")
        .output()
    else {
        return false;
    };

    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .to_lowercase()
            .contains(&needle.to_lowercase())
}

#[cfg(target_os = "windows")]
pub(super) fn windows_service_exists(service_name: &str) -> bool {
    hidden_command(Path::new("sc.exe"))
        .arg("query")
        .arg(service_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
pub(super) fn has_macfuse() -> bool {
    [
        "/Library/Filesystems/macfuse.fs",
        "/Library/Filesystems/osxfuse.fs",
    ]
    .iter()
    .any(|path| Path::new(path).exists())
}
