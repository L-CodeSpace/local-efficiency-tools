/*
 * 核心职责：通过 macOS NetFS 挂载 SMB，并维护应用可见入口链接。
 * 能力边界：真实挂载使用唯一 session 目录，不覆盖用户已有文件或目录。
 */

use super::super::*;
use super::common::qualified_username;
use crate::modules::state::NativeSmbMount;
use core_foundation::{
    array::CFArrayRef,
    base::{kCFAllocatorDefault, CFRelease, TCFType},
    string::CFString,
    url::{CFURLCreateWithString, CFURL},
};
use std::{os::unix::fs::symlink, ptr};

pub(super) fn repair_workspace(
    app: &AppHandle,
    _connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    let base = super::super::storage::app_rclone_dir(app)?
        .join("native-smb")
        .join(&workspace.id);
    for binding in &workspace.bindings {
        let Some(display) = binding.mount_point.as_deref().map(Path::new) else {
            continue;
        };
        let Ok(target) = fs::read_link(display) else {
            continue;
        };
        if !target.starts_with(&base) {
            return Err(AppError::new(
                "mount_target_occupied",
                "macOS SMB 入口不是本应用创建的链接",
            )
            .with_detail(display.to_string_lossy().to_string()));
        }
        unmount_item(
            app,
            &NativeSmbMount {
                remote: String::new(),
                target,
                display_target: Some(display.to_path_buf()),
            },
        );
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
    prepare_display_path(&display)?;
    let actual = super::super::storage::app_rclone_dir(app)?
        .join("native-smb")
        .join(&workspace.id)
        .join(Uuid::new_v4().simple().to_string())
        .join(&binding.id);
    fs::create_dir_all(&actual)?;

    let remote = format!(
        "smb://{}/{}",
        connection.host,
        percent_encode_path(&binding.remote_path)
    );
    let remote_string = CFString::new(&remote);
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
    let remote_url = unsafe { CFURL::wrap_under_create_rule(remote_ref) };
    let mount_url = CFURL::from_path(&actual, true)
        .ok_or_else(|| AppError::new("mount_target_invalid", "创建 macOS SMB 挂载目录 URL 失败"))?;
    let username = CFString::new(&qualified_username(connection));
    let password = CFString::new(connection.password.as_deref().unwrap_or(""));
    let mut mountpoints: CFArrayRef = ptr::null();
    // SAFETY：所有 CF 对象在同步调用期间有效；mountpoints 按 Create Rule 释放。
    let status = unsafe {
        NetFSMountURLSync(
            remote_url.as_concrete_TypeRef(),
            mount_url.as_concrete_TypeRef(),
            username.as_concrete_TypeRef(),
            password.as_concrete_TypeRef(),
            ptr::null(),
            ptr::null(),
            &mut mountpoints,
        )
    };
    if !mountpoints.is_null() {
        unsafe { CFRelease(mountpoints.cast()) };
    }
    if status != 0 {
        let _ = fs::remove_dir_all(&actual);
        return Err(
            AppError::new("mount_smb_connect_failed", "macOS SMB 挂载失败")
                .with_detail(format!("NetFSMountURLSync status {}", status)),
        );
    }
    symlink(&actual, &display).map_err(|error| {
        AppError::new("mount_symlink_failed", "创建 macOS SMB 可见入口失败")
            .with_detail(error.to_string())
    })?;
    Ok(NativeSmbMount {
        remote,
        target: actual,
        display_target: Some(display),
    })
}

pub(super) fn unmount_item(_app: &AppHandle, mount: &NativeSmbMount) {
    if let Some(display) = mount.display_target.as_deref() {
        if matches!(fs::read_link(display), Ok(target) if target == mount.target) {
            let _ = fs::remove_file(display);
        }
    }
    let target = mount.target.to_string_lossy().to_string();
    let _ = Command::new("/usr/sbin/diskutil")
        .args(["unmount", "force", target.as_str()])
        .output();
    let _ = fs::remove_dir_all(&mount.target);
}

fn prepare_display_path(display: &Path) -> AppResult<()> {
    if let Some(parent) = display.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::symlink_metadata(display) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(display)?,
        Ok(_) => {
            return Err(AppError::new(
                "mount_target_exists",
                "macOS SMB 可见入口已存在且不是本应用链接",
            )
            .with_detail(display.to_string_lossy().to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
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
