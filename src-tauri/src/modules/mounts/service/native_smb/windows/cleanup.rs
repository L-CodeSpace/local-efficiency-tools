/*
 * 核心职责：枚举并清理当前 Windows 用户会话中指定主机的全部 SMB 映射。
 * 业务痛点：应用重启或状态丢失后，残留网络盘不能再依赖内存会话释放。
 * 能力边界：只操作精确匹配主机的 WNet 资源，不读取或修改连接凭据。
 */

use crate::{
    modules::mounts::dto::SmbMappingCleanupItem,
    shared::error::{AppError, AppResult},
};
use std::{collections::HashSet, ffi::c_void, mem::size_of, ptr, slice};
use windows_sys::Win32::{
    Foundation::{
        ERROR_BAD_NET_NAME, ERROR_MORE_DATA, ERROR_NOT_CONNECTED, ERROR_NO_MORE_ITEMS, HANDLE,
        NO_ERROR,
    },
    NetworkManagement::WNet::{
        WNetCancelConnection2W, WNetCloseEnum, WNetEnumResourceW, WNetOpenEnumW,
        CONNECT_UPDATE_PROFILE, NETRESOURCEW, RESOURCETYPE_DISK, RESOURCE_CONNECTED,
        RESOURCE_REMEMBERED,
    },
};

const INITIAL_ENUM_BUFFER_BYTES: u32 = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct NetworkResource {
    local_name: Option<String>,
    remote_name: String,
}

struct EnumHandle(HANDLE);

impl Drop for EnumHandle {
    fn drop(&mut self) {
        // SAFETY：句柄只由 WNetOpenEnumW 创建，并且只在此处关闭一次。
        unsafe {
            WNetCloseEnum(self.0);
        }
    }
}

pub(super) fn cleanup_host(host: &str) -> AppResult<Vec<SmbMappingCleanupItem>> {
    let host = normalized_host(host).ok_or_else(|| {
        AppError::new(
            "mount_smb_cleanup_host_invalid",
            "用于清理 SMB 的主机地址无效",
        )
    })?;
    let resources = enumerate_host_resources(&host)?;
    let mut removed = Vec::new();
    let mut failures = Vec::new();
    for resource in resources {
        let target = resource
            .local_name
            .as_deref()
            .unwrap_or(&resource.remote_name);
        let result = cancel_connection(target);
        match result {
            NO_ERROR => removed.push(resource),
            ERROR_NOT_CONNECTED => {}
            code => failures.push(format!(
                "{} -> error {} ({})",
                target,
                code,
                std::io::Error::from_raw_os_error(code as i32)
            )),
        }
    }

    let ipc_remote = format!("\\\\{}\\IPC$", host);
    let ipc_result = cancel_connection(&ipc_remote);
    if ipc_result == NO_ERROR {
        removed.push(NetworkResource {
            local_name: None,
            remote_name: ipc_remote,
        });
    } else if !matches!(ipc_result, ERROR_NOT_CONNECTED | ERROR_BAD_NET_NAME) {
        failures.push(format!(
            "{} -> error {} ({})",
            ipc_remote,
            ipc_result,
            std::io::Error::from_raw_os_error(ipc_result as i32)
        ));
    }

    let remaining = enumerate_host_resources(&host)?;
    if !remaining.is_empty() {
        let residual = remaining
            .iter()
            .map(resource_label)
            .collect::<Vec<_>>()
            .join(", ");
        let mut detail = format!("residualMappings={}", residual);
        if !failures.is_empty() {
            detail.push_str("; failures=");
            detail.push_str(&failures.join(" | "));
        }
        return Err(AppError::new(
            "mount_smb_host_cleanup_partial",
            "部分 Windows SMB 映射未能清理",
        )
        .with_detail(detail));
    }

    deduplicate(&mut removed);
    Ok(removed
        .into_iter()
        .map(|resource| SmbMappingCleanupItem {
            local_name: resource.local_name,
            remote_name: resource.remote_name,
        })
        .collect())
}

fn enumerate_host_resources(host: &str) -> AppResult<Vec<NetworkResource>> {
    let mut resources = enumerate_scope(RESOURCE_CONNECTED)?;
    resources.extend(enumerate_scope(RESOURCE_REMEMBERED)?);
    resources.retain(|resource| remote_matches_host(&resource.remote_name, host));
    deduplicate(&mut resources);
    Ok(resources)
}

fn enumerate_scope(scope: u32) -> AppResult<Vec<NetworkResource>> {
    let mut raw_handle = ptr::null_mut();
    // SAFETY：输出句柄指针有效，枚举根资源时 lpNetResource 必须为空。
    let open_result =
        unsafe { WNetOpenEnumW(scope, RESOURCETYPE_DISK, 0, ptr::null(), &mut raw_handle) };
    if open_result != NO_ERROR {
        return Err(wnet_error(
            "mount_smb_cleanup_enum_open_failed",
            "打开 Windows SMB 资源枚举失败",
            "WNetOpenEnumW",
            open_result,
        ));
    }
    let _handle = EnumHandle(raw_handle);
    let mut output = Vec::new();
    let mut requested_bytes = INITIAL_ENUM_BUFFER_BYTES;
    loop {
        let word_count = requested_bytes as usize / size_of::<usize>() + 1;
        let mut buffer = vec![0usize; word_count];
        let mut buffer_bytes = (buffer.len() * size_of::<usize>()) as u32;
        let mut count = u32::MAX;
        // SAFETY：buffer 按指针宽度对齐且容量通过 buffer_bytes 准确传递。
        let result = unsafe {
            WNetEnumResourceW(
                raw_handle,
                &mut count,
                buffer.as_mut_ptr().cast::<c_void>(),
                &mut buffer_bytes,
            )
        };
        if result == ERROR_NO_MORE_ITEMS {
            break;
        }
        if result == ERROR_MORE_DATA {
            requested_bytes = buffer_bytes.max(requested_bytes.saturating_mul(2));
            continue;
        }
        if result != NO_ERROR {
            return Err(wnet_error(
                "mount_smb_cleanup_enum_failed",
                "枚举 Windows SMB 资源失败",
                "WNetEnumResourceW",
                result,
            ));
        }
        // SAFETY：WNetEnumResourceW 已在对齐缓冲区起始处写入 count 个 NETRESOURCEW。
        let items = unsafe {
            slice::from_raw_parts(buffer.as_ptr().cast::<NETRESOURCEW>(), count as usize)
        };
        for item in items {
            let remote_name = wide_text(item.lpRemoteName);
            if remote_name.is_empty() {
                continue;
            }
            let local_name = wide_text(item.lpLocalName);
            output.push(NetworkResource {
                local_name: (!local_name.is_empty()).then_some(local_name),
                remote_name,
            });
        }
        requested_bytes = INITIAL_ENUM_BUFFER_BYTES;
    }
    Ok(output)
}

fn cancel_connection(target: &str) -> u32 {
    let target = to_wide(target);
    // SAFETY：target 是调用期间有效且以 NUL 结尾的 UTF-16 字符串。
    unsafe { WNetCancelConnection2W(target.as_ptr(), CONNECT_UPDATE_PROFILE, 1) }
}

fn remote_matches_host(remote: &str, host: &str) -> bool {
    unc_host(remote)
        .and_then(normalized_host)
        .is_some_and(|value| value.eq_ignore_ascii_case(host))
}

fn unc_host(remote: &str) -> Option<&str> {
    remote
        .trim()
        .strip_prefix("\\\\")?
        .split('\\')
        .next()
        .filter(|value| !value.is_empty())
}

fn normalized_host(host: &str) -> Option<String> {
    let host = host
        .trim()
        .trim_start_matches(['\\', '/'])
        .trim_end_matches('.')
        .trim_matches(['[', ']']);
    (!host.is_empty() && !host.contains(['\\', '/'])).then(|| host.to_string())
}

fn deduplicate(resources: &mut Vec<NetworkResource>) {
    let mut seen = HashSet::new();
    resources.retain(|resource| {
        seen.insert(format!(
            "{}\0{}",
            resource
                .local_name
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase(),
            resource.remote_name.to_ascii_lowercase()
        ))
    });
}

fn resource_label(resource: &NetworkResource) -> String {
    match resource.local_name.as_deref() {
        Some(local) => format!("{}={}", local, resource.remote_name),
        None => resource.remote_name.clone(),
    }
}

fn wnet_error(code: &str, message: &str, operation: &str, system_code: u32) -> AppError {
    AppError::new(code, message).with_detail(format!(
        "{} error {}: {}",
        operation,
        system_code,
        std::io::Error::from_raw_os_error(system_code as i32)
    ))
}

fn wide_text(value: *mut u16) -> String {
    if value.is_null() {
        return String::new();
    }
    let mut length = 0usize;
    // SAFETY：WNet 返回的字符串位于当前枚举缓冲区中，并保证以 NUL 结尾。
    unsafe {
        while *value.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(slice::from_raw_parts(value, length))
    }
}

fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

#[cfg(test)]
#[path = "cleanup/tests.rs"]
mod tests;
