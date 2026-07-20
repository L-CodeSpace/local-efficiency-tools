/*
 * 核心职责：验证 Windows SMB 登录身份协商、错误重试和密码内存清理规则。
 * 能力边界：只测试纯规则，不调用真实 WNet 挂载 API。
 */

use super::*;
use crate::modules::mounts::dto::TransportPreference;

fn connection(domain: Option<&str>, mode: WindowsSmbAuthMode) -> RemoteConnection {
    RemoteConnection {
        id: "connection".to_string(),
        name: "NAS".to_string(),
        host: "192.168.88.186".to_string(),
        username: "SUSU".to_string(),
        password: Some("secret".to_string()),
        domain: domain.map(str::to_string),
        ftp_port: 21,
        smb_port: 445,
        tls_mode: None,
        no_check_certificate: false,
        transport_preference: TransportPreference::Auto,
        windows_auth_mode: mode,
        created_at: 0,
        updated_at: 0,
    }
}

fn modes(items: &[AuthCandidate]) -> Vec<ResolvedAuthMode> {
    items.iter().map(|item| item.mode).collect()
}

#[test]
fn workgroup_auto_prefers_host_identity() {
    let result = candidates(
        &connection(Some("WORKGROUP"), WindowsSmbAuthMode::Auto),
        None,
    )
    .unwrap();
    assert_eq!(
        modes(&result),
        vec![
            ResolvedAuthMode::Host,
            ResolvedAuthMode::Plain,
            ResolvedAuthMode::Domain
        ]
    );
    assert_eq!(result[0].username, "192.168.88.186\\SUSU");
    assert_eq!(result[1].username, "SUSU");
    assert_eq!(result[2].username, "WORKGROUP\\SUSU");
}

#[test]
fn real_domain_auto_prefers_qualified_identity() {
    let result = candidates(&connection(Some("CORP"), WindowsSmbAuthMode::Auto), None).unwrap();
    assert_eq!(
        modes(&result),
        vec![
            ResolvedAuthMode::Domain,
            ResolvedAuthMode::Host,
            ResolvedAuthMode::Plain
        ]
    );
}

#[test]
fn manual_modes_generate_only_selected_identity() {
    let plain = candidates(
        &connection(Some("WORKGROUP"), WindowsSmbAuthMode::Plain),
        None,
    )
    .unwrap();
    let domain = candidates(
        &connection(Some("WORKGROUP"), WindowsSmbAuthMode::Domain),
        None,
    )
    .unwrap();
    assert_eq!(modes(&plain), vec![ResolvedAuthMode::Plain]);
    assert_eq!(modes(&domain), vec![ResolvedAuthMode::Domain]);
}

#[test]
fn retries_only_identity_authentication_errors() {
    assert!(is_retryable_auth_error(ERROR_INVALID_PASSWORD));
    assert!(is_retryable_auth_error(ERROR_LOGON_FAILURE));
    assert!(is_retryable_auth_error(ERROR_BAD_USERNAME));
    assert!(!is_retryable_auth_error(ERROR_SESSION_CREDENTIAL_CONFLICT));
    assert!(!is_retryable_auth_error(ERROR_EXTENDED_ERROR));
}

#[test]
fn empty_password_fails_before_wnet_call() {
    let mut value = connection(None, WindowsSmbAuthMode::Auto);
    value.password = None;
    assert_eq!(
        required_password(&value).unwrap_err().code,
        "mount_smb_password_required"
    );
}

#[test]
fn sensitive_wide_buffer_is_zeroed() {
    let mut value = SensitiveWide::new("secret");
    value.clear();
    assert!(value.0.iter().all(|item| *item == 0));
}
