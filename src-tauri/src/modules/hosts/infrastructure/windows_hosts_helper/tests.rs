/*
 * 核心职责：验证 Windows helper 安全边界。
 * 业务痛点：拆分后仍需覆盖 SID 和 hosts 路径白名单。
 * 能力边界：只包含 helper 单元测试。
 */

#[cfg(test)]
mod tests {
    use crate::modules::hosts::infrastructure::windows_hosts_helper::*;
    use std::path::Path;

    #[test]
    fn allows_only_windows_hosts_path() {
        assert!(is_allowed_hosts_path(Path::new(
            r"C:\Windows\System32\drivers\etc\hosts"
        )));
        assert!(is_allowed_hosts_path(Path::new(
            r"c:/windows/system32/drivers/etc/hosts"
        )));
        assert!(!is_allowed_hosts_path(Path::new(
            r"C:\Windows\System32\drivers\etc\services"
        )));
    }

    #[test]
    fn extracts_sid_from_whoami_csv() {
        assert_eq!(
            extract_sid("\"DESKTOP\\user\",\"S-1-5-21-1-2-3-1001\""),
            Some("S-1-5-21-1-2-3-1001".to_string())
        );
    }
}
