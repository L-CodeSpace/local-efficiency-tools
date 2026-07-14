/*
 * 核心职责：通过 Windows WNet API 挂载、校验和释放 SMB 网络盘。
 * 能力边界：凭据仅在内存中传给系统 API，不写入命令行或日志。
 */

use super::super::*;
use super::common::smb_unc_path;
use crate::modules::state::NativeSmbMount;
use windows_sys::Win32::NetworkManagement::WNet::NETRESOURCEW;

#[path = "windows/auth.rs"]
mod auth;
#[path = "windows/cleanup.rs"]
mod cleanup;

use auth::{
    candidates, connection_error, is_retryable_auth_error, required_password, ResolvedAuthMode,
    SensitiveWide,
};

pub(super) fn cleanup_host(host: &str) -> AppResult<Vec<SmbMappingCleanupItem>> {
    cleanup::cleanup_host(host)
}

pub(super) fn mount_workspace(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<Vec<NativeSmbMount>> {
    let password = required_password(connection)?;
    let mut mounted = Vec::new();
    let mut successful_mode = None;
    for binding in &workspace.bindings {
        match mount_binding(app, connection, binding, password, successful_mode) {
            Ok((item, resolved_mode)) => {
                successful_mode = successful_mode.or(resolved_mode);
                mounted.push(item);
            }
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

pub(super) fn repair_workspace(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    for binding in &workspace.bindings {
        let drive = normalize_drive_letter(binding.drive_letter.as_deref())?;
        let expected = smb_unc_path(&connection.host, &binding.remote_path)?;
        match remote_for_drive(&drive)? {
            Some(remote) if remote.eq_ignore_ascii_case(&expected) => {
                unmount_item(
                    app,
                    &NativeSmbMount {
                        remote,
                        target: PathBuf::from(format!("{}\\", drive)),
                        display_target: None,
                        marker_path: None,
                    },
                );
            }
            Some(remote) => {
                return Err(AppError::new(
                    "mount_drive_occupied",
                    format!("盘符 {} 已被其他网络资源占用", drive),
                )
                .with_detail(remote));
            }
            None => {}
        }
    }
    Ok(())
}

fn mount_binding(
    app: &AppHandle,
    connection: &RemoteConnection,
    binding: &RemoteBinding,
    password: &str,
    successful_mode: Option<ResolvedAuthMode>,
) -> AppResult<(NativeSmbMount, Option<ResolvedAuthMode>)> {
    use windows_sys::Win32::{Foundation::NO_ERROR, NetworkManagement::WNet::RESOURCETYPE_DISK};

    let drive = normalize_drive_letter(binding.drive_letter.as_deref())?;
    let remote = smb_unc_path(&connection.host, &binding.remote_path)?;
    if let Some(existing) = remote_for_drive(&drive)? {
        if existing.eq_ignore_ascii_case(&remote) {
            return Ok((native_mount(remote, &drive), None));
        }
        return Err(AppError::new(
            "mount_drive_occupied",
            format!("盘符 {} 已映射到其他网络资源", drive),
        )
        .with_detail(existing));
    }
    if PathBuf::from(format!("{}\\", drive)).exists() {
        return Err(AppError::new(
            "mount_drive_occupied",
            format!("盘符 {} 已被本地磁盘或其他设备占用", drive),
        ));
    }

    let mut drive_wide = to_wide(&drive);
    let mut remote_wide = to_wide(&remote);
    let resource = NETRESOURCEW {
        dwType: RESOURCETYPE_DISK,
        lpLocalName: drive_wide.as_mut_ptr(),
        lpRemoteName: remote_wide.as_mut_ptr(),
        ..NETRESOURCEW::default()
    };
    let auth_candidates = candidates(connection, successful_mode)?;
    let mut attempted = Vec::new();
    for (index, candidate) in auth_candidates.iter().enumerate() {
        let result = connect_mapping(&resource, &candidate.username, password);
        attempted.push(candidate.mode);
        observability::emit_info(
            app,
            format!(
                "Windows SMB 登录尝试：mode={} code={}",
                candidate.mode.label(),
                result
            ),
        );
        if result == NO_ERROR {
            let mount = native_mount(remote.clone(), &drive);
            if remote_for_drive(&drive)?
                .as_deref()
                .is_none_or(|value| !value.eq_ignore_ascii_case(&remote))
            {
                unmount_item(app, &mount);
                return Err(AppError::new(
                    "mount_smb_verify_failed",
                    "Windows SMB 挂载完成后校验失败",
                ));
            }
            return Ok((mount, Some(candidate.mode)));
        }
        let has_next = index + 1 < auth_candidates.len();
        if !has_next || !is_retryable_auth_error(result) {
            return Err(connection_error(result, &attempted));
        }
    }
    Err(AppError::new(
        "mount_smb_auth_candidates_empty",
        "没有可用的 Windows SMB 登录方式",
    ))
}

fn native_mount(remote: String, drive: &str) -> NativeSmbMount {
    NativeSmbMount {
        remote,
        target: PathBuf::from(format!("{}\\", drive)),
        display_target: None,
        marker_path: None,
    }
}

fn connect_mapping(resource: &NETRESOURCEW, username: &str, password: &str) -> u32 {
    use windows_sys::Win32::NetworkManagement::WNet::WNetAddConnection2W;
    let username_wide = to_wide(username);
    let mut password_wide = SensitiveWide::new(password);
    // SAFETY：所有 UTF-16 缓冲区在调用期间保持有效且以 NUL 结尾。
    let result =
        unsafe { WNetAddConnection2W(resource, password_wide.as_ptr(), username_wide.as_ptr(), 0) };
    password_wide.clear();
    result
}

pub(super) fn unmount_item(_app: &AppHandle, mount: &NativeSmbMount) {
    use windows_sys::Win32::NetworkManagement::WNet::WNetCancelConnection2W;
    let local = mount
        .target
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    let local_wide = to_wide(&local);
    // SAFETY：local_wide 是调用期间有效的 NUL 结尾 UTF-16 字符串。
    unsafe {
        WNetCancelConnection2W(local_wide.as_ptr(), 0, 1);
    }
}

fn remote_for_drive(drive: &str) -> AppResult<Option<String>> {
    use windows_sys::Win32::{
        Foundation::{ERROR_MORE_DATA, ERROR_NOT_CONNECTED, NO_ERROR},
        NetworkManagement::WNet::WNetGetConnectionW,
    };
    let local = to_wide(drive);
    let mut capacity = 512u32;
    let mut buffer = vec![0u16; capacity as usize];
    // SAFETY：buffer 可写且 capacity 与其长度一致，local 为 NUL 结尾 UTF-16 字符串。
    let mut result =
        unsafe { WNetGetConnectionW(local.as_ptr(), buffer.as_mut_ptr(), &mut capacity) };
    if result == ERROR_MORE_DATA {
        buffer.resize(capacity as usize, 0);
        // SAFETY：扩容后继续满足 WNetGetConnectionW 的缓冲区约束。
        result = unsafe { WNetGetConnectionW(local.as_ptr(), buffer.as_mut_ptr(), &mut capacity) };
    }
    if result == ERROR_NOT_CONNECTED {
        return Ok(None);
    }
    if result != NO_ERROR {
        return Err(
            AppError::new("mount_drive_query_failed", "查询 Windows 网络盘符失败")
                .with_detail(format!("WNetGetConnectionW error {}", result)),
        );
    }
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    Ok(Some(String::from_utf16_lossy(&buffer[..length])))
}

fn normalize_drive_letter(value: Option<&str>) -> AppResult<String> {
    let value = value.unwrap_or("").trim().trim_end_matches(['\\', '/']);
    let mut chars = value.chars();
    let Some(letter) = chars.next().filter(|value| value.is_ascii_alphabetic()) else {
        return Err(AppError::new(
            "mount_drive_required",
            "Windows SMB 挂载需要有效盘符",
        ));
    };
    if !matches!(chars.next(), Some(':')) || chars.next().is_some() {
        return Err(AppError::new(
            "mount_drive_invalid",
            "Windows SMB 盘符格式无效",
        ));
    }
    Ok(format!("{}:", letter.to_ascii_uppercase()))
}

fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}
