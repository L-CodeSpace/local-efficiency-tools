/*
 * 核心职责：生成 Windows SMB 登录身份候选并解释 WNet 认证错误。
 * 业务痛点：NAS 本地账户可能要求主机前缀并拒绝 WORKGROUP 前缀，需要受控协商用户名格式。
 * 能力边界：不创建网络盘，不持久化凭据，不记录用户名或密码。
 */

use crate::{
    modules::mounts::dto::{RemoteConnection, WindowsSmbAuthMode},
    shared::error::{AppError, AppResult},
};
use windows_sys::Win32::Foundation::{
    ERROR_BAD_USERNAME, ERROR_EXTENDED_ERROR, ERROR_INVALID_PASSWORD, ERROR_LOGON_FAILURE,
    ERROR_SESSION_CREDENTIAL_CONFLICT, NO_ERROR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResolvedAuthMode {
    Host,
    Plain,
    Domain,
}

impl ResolvedAuthMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Plain => "plain",
            Self::Domain => "domain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AuthCandidate {
    pub(super) mode: ResolvedAuthMode,
    pub(super) username: String,
}

pub(super) struct SensitiveWide(Vec<u16>);

impl SensitiveWide {
    pub(super) fn new(value: &str) -> Self {
        use std::os::windows::ffi::OsStrExt;
        Self(
            std::ffi::OsStr::new(value)
                .encode_wide()
                .chain(Some(0))
                .collect(),
        )
    }

    pub(super) fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    pub(super) fn clear(&mut self) {
        self.0.fill(0);
    }
}

impl Drop for SensitiveWide {
    fn drop(&mut self) {
        self.clear();
    }
}

pub(super) fn required_password(connection: &RemoteConnection) -> AppResult<&str> {
    let password = connection
        .password
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::new(
                "mount_smb_password_required",
                "Windows SMB 挂载需要连接密码",
            )
        })?;
    if password.contains('\0') {
        return Err(AppError::new(
            "mount_smb_password_invalid",
            "Windows SMB 密码包含无效字符",
        ));
    }
    Ok(password)
}

pub(super) fn candidates(
    connection: &RemoteConnection,
    preferred: Option<ResolvedAuthMode>,
) -> AppResult<Vec<AuthCandidate>> {
    let username = connection.username.trim();
    if username.is_empty() || username.contains('\0') {
        return Err(AppError::new(
            "mount_smb_username_invalid",
            "Windows SMB 用户名格式无效",
        ));
    }
    let domain = connection.domain.as_deref().map(str::trim).unwrap_or("");
    let host = connection.host.trim();
    if domain.contains('\0') {
        return Err(AppError::new(
            "mount_smb_domain_invalid",
            "Windows SMB 域格式无效",
        ));
    }

    let modes = if let Some(mode) = preferred {
        vec![mode]
    } else {
        configured_modes(connection.windows_auth_mode, domain)?
    };
    modes
        .into_iter()
        .map(|mode| candidate(mode, username, domain, host))
        .collect()
}

fn configured_modes(
    configured: WindowsSmbAuthMode,
    domain: &str,
) -> AppResult<Vec<ResolvedAuthMode>> {
    match configured {
        WindowsSmbAuthMode::Plain => Ok(vec![ResolvedAuthMode::Plain]),
        WindowsSmbAuthMode::Domain if domain.is_empty() => Err(AppError::new(
            "mount_smb_domain_required",
            "域登录方式需要填写 Windows SMB 域",
        )),
        WindowsSmbAuthMode::Domain => Ok(vec![ResolvedAuthMode::Domain]),
        WindowsSmbAuthMode::Auto if domain.is_empty() => {
            Ok(vec![ResolvedAuthMode::Host, ResolvedAuthMode::Plain])
        }
        WindowsSmbAuthMode::Auto if domain.eq_ignore_ascii_case("WORKGROUP") => Ok(vec![
            ResolvedAuthMode::Host,
            ResolvedAuthMode::Plain,
            ResolvedAuthMode::Domain,
        ]),
        WindowsSmbAuthMode::Auto => Ok(vec![
            ResolvedAuthMode::Domain,
            ResolvedAuthMode::Host,
            ResolvedAuthMode::Plain,
        ]),
    }
}

fn candidate(
    mode: ResolvedAuthMode,
    username: &str,
    domain: &str,
    host: &str,
) -> AppResult<AuthCandidate> {
    let username = match mode {
        ResolvedAuthMode::Host if host.is_empty() || host.contains(['\\', '/']) => {
            return Err(AppError::new(
                "mount_smb_host_invalid",
                "主机登录方式需要有效的 Windows SMB 主机",
            ));
        }
        ResolvedAuthMode::Host => format!("{}\\{}", host, username),
        ResolvedAuthMode::Plain => username.to_string(),
        ResolvedAuthMode::Domain if domain.is_empty() => {
            return Err(AppError::new(
                "mount_smb_domain_required",
                "域登录方式需要填写 Windows SMB 域",
            ));
        }
        ResolvedAuthMode::Domain => format!("{}\\{}", domain, username),
    };
    Ok(AuthCandidate { mode, username })
}

pub(super) fn is_retryable_auth_error(code: u32) -> bool {
    matches!(
        code,
        ERROR_INVALID_PASSWORD | ERROR_LOGON_FAILURE | ERROR_BAD_USERNAME
    )
}

pub(super) fn connection_error(code: u32, attempted: &[ResolvedAuthMode]) -> AppError {
    let (error_code, message) = match code {
        ERROR_INVALID_PASSWORD => (
            "mount_smb_auth_failed",
            "Windows SMB 认证失败；密码正确时通常是登录名格式或 NAS 账户权限不匹配",
        ),
        ERROR_LOGON_FAILURE | ERROR_BAD_USERNAME => {
            ("mount_smb_auth_failed", "Windows SMB 用户名或密码不正确")
        }
        ERROR_SESSION_CREDENTIAL_CONFLICT => (
            "mount_smb_credential_conflict",
            "同一服务器已有其他凭据会话",
        ),
        _ => ("mount_smb_connect_failed", "Windows SMB 挂载失败"),
    };
    let modes = attempted
        .iter()
        .map(|mode| mode.label())
        .collect::<Vec<_>>()
        .join(" -> ");
    let mut detail = format!(
        "WNetAddConnection2W error {}: {}; attemptedModes={}",
        code,
        std::io::Error::from_raw_os_error(code as i32),
        modes
    );
    if code == ERROR_EXTENDED_ERROR {
        if let Some(extended) = extended_error_detail() {
            detail.push_str("; ");
            detail.push_str(&extended);
        }
    }
    AppError::new(error_code, message).with_detail(detail)
}

fn extended_error_detail() -> Option<String> {
    use windows_sys::Win32::NetworkManagement::WNet::WNetGetLastErrorW;
    let mut provider_code = 0u32;
    let mut message = vec![0u16; 512];
    let mut provider = vec![0u16; 256];
    // SAFETY：缓冲区可写且长度与传入容量一致，仅接收网络提供程序错误文本。
    let status = unsafe {
        WNetGetLastErrorW(
            &mut provider_code,
            message.as_mut_ptr(),
            message.len() as u32,
            provider.as_mut_ptr(),
            provider.len() as u32,
        )
    };
    if status != NO_ERROR {
        return Some(format!("WNetGetLastErrorW error {}", status));
    }
    Some(format!(
        "providerError={}; provider={}; providerMessage={}",
        provider_code,
        utf16_text(&provider),
        utf16_text(&message)
    ))
}

fn utf16_text(buffer: &[u16]) -> String {
    let length = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}

#[cfg(test)]
#[path = "auth/tests.rs"]
mod tests;
