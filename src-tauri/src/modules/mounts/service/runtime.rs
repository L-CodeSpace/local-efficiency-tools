/*
 * 核心职责：检测 rclone 运行时和系统依赖。
 * 业务痛点：运行时状态与平台依赖是挂载入口的前置条件。
 * 能力边界：只负责状态展示和依赖探测入口。
 */

#[cfg(target_os = "macos")]
use super::dependencies::has_macfuse;
#[cfg(target_os = "windows")]
use super::dependencies::probe_windows_fuse;
use super::*;
use super::{
    dependencies::current_platform,
    runtime_download::{current_asset, download_rclone, get_rclone_version},
    storage::{
        app_rclone_dir, default_drive_letter, default_mount_dir_name, default_mount_root,
        profiles_path, rclone_binary_path, rclone_config_path,
    },
};

pub(super) const RCLONE_VERSION: &str = "v1.74.3";
#[cfg(target_os = "windows")]
pub(super) const WINFSP_URL: &str = "https://winfsp.dev/";
#[cfg(target_os = "macos")]
pub(super) const MACFUSE_URL: &str = "https://macfuse.github.io/";

#[derive(Clone, Copy)]
pub(super) struct RcloneAsset {
    pub(super) filename: &'static str,
    pub(super) url: &'static str,
    pub(super) sha256: &'static str,
    pub(super) source_name: &'static str,
    pub(super) source_url: &'static str,
}

pub fn runtime_status(app: &AppHandle) -> AppResult<MountRuntimeStatus> {
    let path = rclone_binary_path(app)?;
    let version = if path.exists() {
        get_rclone_version(&path).ok()
    } else {
        None
    };
    let installed = version
        .as_deref()
        .map(|value| value.contains(RCLONE_VERSION))
        .unwrap_or(false);

    let asset = current_asset().ok();
    Ok(MountRuntimeStatus {
        installed,
        version,
        path: path.to_string_lossy().to_string(),
        expected_version: RCLONE_VERSION.to_string(),
        download_required: !installed,
        source_name: asset.map(|asset| asset.source_name.to_string()),
        source_url: asset.map(|asset| asset.source_url.to_string()),
        download_supported: asset.is_some(),
    })
}

pub fn download_runtime(app: &AppHandle) -> AppResult<MountRuntimeStatus> {
    download_rclone(app)?;
    runtime_status(app)
}

pub fn check_dependencies() -> MountDependencyStatus {
    #[cfg(target_os = "windows")]
    {
        let probe = probe_windows_fuse();
        let installed = probe.winfsp;
        return MountDependencyStatus {
            supported: true,
            ready: installed,
            dependency_name: "WinFsp".to_string(),
            installed,
            install_url: if installed {
                None
            } else {
                Some(WINFSP_URL.to_string())
            },
            message: if installed {
                "WinFsp 已安装，可以使用 rclone mount。".to_string()
            } else if probe.sshfs_win {
                "检测到 SSHFS-Win，但未检测到 WinFsp 核心运行时。SSHFS-Win 不是 rclone mount 所需的 WinFsp 运行时，请安装 WinFsp 后再启用挂载。"
                    .to_string()
            } else {
                "未检测到 WinFsp。请从官方站点安装后再启用挂载。".to_string()
            },
        };
    }

    #[cfg(target_os = "macos")]
    {
        let installed = has_macfuse();
        return MountDependencyStatus {
            supported: true,
            ready: installed,
            dependency_name: "macFUSE".to_string(),
            installed,
            install_url: if installed {
                None
            } else {
                Some(MACFUSE_URL.to_string())
            },
            message: if installed {
                "macFUSE 已安装，可以使用 rclone mount。".to_string()
            } else {
                "未检测到 macFUSE。请从官方站点安装后再启用挂载。".to_string()
            },
        };
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        MountDependencyStatus {
            supported: false,
            ready: false,
            dependency_name: "FUSE".to_string(),
            installed: false,
            install_url: None,
            message: "当前版本仅支持 Windows 和 macOS 的 rclone 挂载。".to_string(),
        }
    }
}

pub fn ui_context(app: &AppHandle) -> AppResult<MountUiContext> {
    let (default_root, uses_desktop) = default_mount_root(app)?;
    let default_example = default_root.join(default_mount_dir_name("nas"));
    let default_drive_letter = default_drive_letter(app);
    let config_dir = app_rclone_dir(app)?;
    let profile_config_path = profiles_path(app)?;
    let rclone_config_path = rclone_config_path(app)?;

    Ok(MountUiContext {
        platform: current_platform(),
        default_mount_root: default_root.to_string_lossy().to_string(),
        default_mount_example: default_example.to_string_lossy().to_string(),
        default_drive_letter,
        config_dir: config_dir.to_string_lossy().to_string(),
        profile_config_path: profile_config_path.to_string_lossy().to_string(),
        rclone_config_path: rclone_config_path.to_string_lossy().to_string(),
        supports_drive_letter: cfg!(target_os = "windows"),
        message: if cfg!(target_os = "windows") {
            "Windows 默认推荐使用盘符并作为网络驱动器挂载。".to_string()
        } else if uses_desktop {
            "留空时会在桌面创建与配置名称同名的挂载目录。".to_string()
        } else {
            "未能读取系统桌面目录，留空时会使用应用数据目录中的 rclone/mounts。".to_string()
        },
    })
}
