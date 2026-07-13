/*
 * 核心职责：通过 macOS NetFS 挂载 SMB，并维护应用可见入口和会话标记。
 * 能力边界：让系统选择 /Volumes 下的真实挂载点，不覆盖或递归删除用户目录。
 */

use super::super::*;
use super::common::qualified_username;
use crate::modules::state::NativeSmbMount;
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{kCFAllocatorDefault, CFGetTypeID, TCFType},
    string::{CFString, CFStringGetTypeID, CFStringRef},
    url::{CFURLCreateWithString, CFURLGetTypeID, CFURLRef, CFURL},
};
use std::{os::unix::fs::symlink, ptr};

pub(super) fn repair_workspace(
    app: &AppHandle,
    _connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    for binding in &workspace.bindings {
        cleanup_stale_binding(app, workspace, binding)?;
    }
    Ok(())
}

pub(super) fn mount_binding(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
    binding: &RemoteBinding,
) -> AppResult<NativeSmbMount> {
    let display = binding
        .mount_point
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| AppError::new("mount_point_required", "macOS SMB 挂载需要本地入口目录"))?;
    cleanup_stale_binding(app, workspace, binding)?;
    prepare_display_path(&display)?;

    let remote = format!(
        "smb://{}/{}",
        connection.host,
        percent_encode_path(&binding.remote_path)
    );
    let remote_url = create_remote_url(&remote)?;
    let username = CFString::new(&qualified_username(connection));
    let password = CFString::new(connection.password.as_deref().unwrap_or(""));
    let mut mountpoints_ref: CFArrayRef = ptr::null();
    // SAFETY：所有 CF 对象在同步调用期间有效；mountpath 为空时由 NetFS 选择系统挂载点。
    let status = unsafe {
        NetFSMountURLSync(
            remote_url.as_concrete_TypeRef(),
            ptr::null(),
            username.as_concrete_TypeRef(),
            password.as_concrete_TypeRef(),
            ptr::null(),
            ptr::null(),
            &mut mountpoints_ref,
        )
    };
    let mountpoints: Option<CFArray> = (!mountpoints_ref.is_null())
        .then(|| unsafe { CFArray::wrap_under_create_rule(mountpoints_ref) });
    if status != 0 {
        return Err(netfs_mount_error(status, &remote));
    }
    let actual = mountpoints
        .as_ref()
        .and_then(first_mountpoint_path)
        .ok_or_else(|| {
            AppError::new(
                "mount_smb_mountpoint_missing",
                "macOS SMB 挂载成功但未返回实际挂载点",
            )
            .with_detail(remote.clone())
        })?;

    if let Err(error) = symlink(&actual, &display) {
        force_unmount(&actual);
        return Err(
            AppError::new("mount_symlink_failed", "创建 macOS SMB 可见入口失败")
                .with_detail(error.to_string()),
        );
    }
    let marker = binding_marker_path(app, workspace, binding)?;
    if let Err(error) = write_marker(&marker, &actual) {
        let _ = fs::remove_file(&display);
        force_unmount(&actual);
        return Err(error);
    }
    Ok(NativeSmbMount {
        remote,
        target: actual,
        display_target: Some(display),
        marker_path: Some(marker),
    })
}

fn first_mountpoint_path(items: &CFArray) -> Option<PathBuf> {
    let value = *items.get(0)?;
    if value.is_null() {
        return None;
    }
    // SAFETY：数组元素由 NetFS 返回并由 CFArray 持有；先检查运行时类型再包装引用。
    unsafe {
        let type_id = CFGetTypeID(value);
        if type_id == CFURLGetTypeID() {
            return CFURL::wrap_under_get_rule(value as CFURLRef).to_path();
        }
        if type_id == CFStringGetTypeID() {
            let path = CFString::wrap_under_get_rule(value as CFStringRef).to_string();
            return (!path.trim().is_empty()).then(|| PathBuf::from(path));
        }
    }
    None
}

pub(super) fn unmount_item(_app: &AppHandle, mount: &NativeSmbMount) {
    if let Some(display) = mount.display_target.as_deref() {
        if matches!(fs::read_link(display), Ok(target) if target == mount.target) {
            let _ = fs::remove_file(display);
        }
    }
    force_unmount(&mount.target);
    if let Some(marker) = mount.marker_path.as_deref() {
        let _ = fs::remove_file(marker);
    }
}

fn create_remote_url(remote: &str) -> AppResult<CFURL> {
    let remote_string = CFString::new(remote);
    // SAFETY：CFURLCreateWithString 遵守 Create Rule，返回值交给 CFURL 管理生命周期。
    let remote_ref = unsafe {
        CFURLCreateWithString(
            kCFAllocatorDefault,
            remote_string.as_concrete_TypeRef(),
            ptr::null(),
        )
    };
    if remote_ref.is_null() {
        return Err(AppError::new(
            "mount_smb_url_invalid",
            "创建 macOS SMB URL 失败",
        ));
    }
    Ok(unsafe { CFURL::wrap_under_create_rule(remote_ref) })
}

fn cleanup_stale_binding(
    app: &AppHandle,
    workspace: &MountWorkspace,
    binding: &RemoteBinding,
) -> AppResult<()> {
    let marker = binding_marker_path(app, workspace, binding)?;
    let Ok(recorded) = fs::read_to_string(&marker) else {
        return Ok(());
    };
    let target = PathBuf::from(recorded.trim());
    if target.starts_with("/Volumes/") {
        force_unmount(&target);
    }
    if let Some(display) = binding.mount_point.as_deref().map(Path::new) {
        if matches!(fs::read_link(display), Ok(link) if link == target) {
            fs::remove_file(display)?;
        }
    }
    let _ = fs::remove_file(marker);
    Ok(())
}

fn binding_marker_path(
    app: &AppHandle,
    workspace: &MountWorkspace,
    binding: &RemoteBinding,
) -> AppResult<PathBuf> {
    Ok(super::super::storage::app_rclone_dir(app)?
        .join("native-smb-sessions")
        .join(&workspace.id)
        .join(format!("{}.mount", binding.id)))
}

fn write_marker(marker: &Path, target: &Path) -> AppResult<()> {
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(marker, target.to_string_lossy().as_bytes()).map_err(|error| {
        AppError::new("mount_session_marker_failed", "记录 macOS SMB 会话失败")
            .with_detail(error.to_string())
    })
}

fn prepare_display_path(display: &Path) -> AppResult<()> {
    if let Some(parent) = display.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::symlink_metadata(display) {
        Ok(_) => Err(AppError::new(
            "mount_target_exists",
            "macOS SMB 可见入口已存在且不属于当前挂载会话",
        )
        .with_detail(display.to_string_lossy().to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn force_unmount(target: &Path) {
    let target = target.to_string_lossy().to_string();
    let diskutil = Command::new("/usr/sbin/diskutil")
        .args(["unmount", "force", target.as_str()])
        .output();
    if !matches!(diskutil, Ok(output) if output.status.success()) {
        let _ = Command::new("/sbin/umount")
            .args(["-f", target.as_str()])
            .output();
    }
}

fn netfs_mount_error(status: i32, remote: &str) -> AppError {
    let description = if status > 0 {
        std::io::Error::from_raw_os_error(status).to_string()
    } else {
        "NetFS/NetAuth 返回系统状态错误".to_string()
    };
    AppError::new("mount_smb_connect_failed", "macOS SMB 挂载失败").with_detail(format!(
        "NetFSMountURLSync status {} ({}), remote={}",
        status, description, remote
    ))
}

fn percent_encode_path(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.trim_matches('/').as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

#[link(name = "NetFS", kind = "framework")]
extern "C" {
    fn NetFSMountURLSync(
        url: core_foundation::url::CFURLRef,
        mountpath: core_foundation::url::CFURLRef,
        user: core_foundation::string::CFStringRef,
        passwd: core_foundation::string::CFStringRef,
        open_options: core_foundation::dictionary::CFDictionaryRef,
        mount_options: core_foundation::dictionary::CFDictionaryRef,
        mountpoints: *mut core_foundation::array::CFArrayRef,
    ) -> i32;
}
