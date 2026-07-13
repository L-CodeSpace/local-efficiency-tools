/*
 * 核心职责：通过 Windows WNet API 挂载、校验和释放 SMB 网络盘。
 * 能力边界：凭据仅在内存中传给系统 API，不写入命令行或日志。
 */

use super::super::*;
use super::common::{qualified_username, smb_unc_path};
use crate::modules::state::NativeSmbMount;

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

pub(super) fn mount_binding(
    _app: &AppHandle,
    connection: &RemoteConnection,
    _workspace: &MountWorkspace,
    binding: &RemoteBinding,
) -> AppResult<NativeSmbMount> {
    use windows_sys::Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::WNet::{WNetAddConnection2W, NETRESOURCEW, RESOURCETYPE_DISK},
    };

    let drive = normalize_drive_letter(binding.drive_letter.as_deref())?;
    let remote = smb_unc_path(&connection.host, &binding.remote_path)?;
    if let Some(existing) = remote_for_drive(&drive)? {
        if existing.eq_ignore_ascii_case(&remote) {
            return Ok(NativeSmbMount {
                remote,
                target: PathBuf::from(format!("{}\\", drive)),
                display_target: None,
            });
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
    let username_wide = to_wide(&qualified_username(connection));
    let mut password_wide = to_wide(connection.password.as_deref().unwrap_or(""));
    let resource = NETRESOURCEW {
        dwType: RESOURCETYPE_DISK,
        lpLocalName: drive_wide.as_mut_ptr(),
        lpRemoteName: remote_wide.as_mut_ptr(),
        ..NETRESOURCEW::default()
    };
    // SAFETY：所有 UTF-16 缓冲区在调用期间保持有效且以 NUL 结尾。
    let result = unsafe {
        WNetAddConnection2W(
            &resource,
            pointer_or_null(&password_wide),
            pointer_or_null(&username_wide),
            0,
        )
    };
    password_wide.fill(0);
    if result != NO_ERROR {
        return Err(
            AppError::new("mount_smb_connect_failed", "Windows SMB 挂载失败")
                .with_detail(format!("WNetAddConnection2W error {}", result)),
        );
    }
    if remote_for_drive(&drive)?
        .as_deref()
        .is_none_or(|value| !value.eq_ignore_ascii_case(&remote))
    {
        return Err(AppError::new(
            "mount_smb_verify_failed",
            "Windows SMB 挂载完成后校验失败",
        ));
    }
    Ok(NativeSmbMount {
        remote,
        target: PathBuf::from(format!("{}\\", drive)),
        display_target: None,
    })
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

fn pointer_or_null(value: &[u16]) -> *const u16 {
    if value.len() > 1 {
        value.as_ptr()
    } else {
        std::ptr::null()
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}
