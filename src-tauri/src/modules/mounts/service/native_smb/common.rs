/*
 * 核心职责：提供跨平台 SMB 路径与账号规范化规则。
 * 能力边界：只做纯数据转换，不访问系统挂载 API。
 */

use super::super::*;

pub(super) fn smb_unc_path(host: &str, remote_path: &str) -> AppResult<String> {
    let share = remote_path.trim().trim_matches(['/', '\\']);
    if share.is_empty() || share.contains(['\r', '\n']) {
        return Err(AppError::new("mount_smb_share_invalid", "SMB 共享路径无效"));
    }
    Ok(format!(r"\\{}\{}", host.trim(), share.replace('/', "\\")))
}

pub(super) fn qualified_username(connection: &RemoteConnection) -> String {
    match connection
        .domain
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(domain) => format!(r"{}\{}", domain, connection.username),
        None => connection.username.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::smb_unc_path;

    #[test]
    fn smb_unc_path_preserves_nested_share_path() {
        assert_eq!(
            smb_unc_path("192.168.1.2", "/Video/素材").unwrap(),
            r"\\192.168.1.2\Video\素材"
        );
    }
}
