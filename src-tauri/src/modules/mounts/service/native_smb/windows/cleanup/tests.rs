/*
 * 核心职责：验证 SMB 主机精确匹配与跨枚举范围去重规则。
 * 能力边界：只测试纯数据处理，不断开真实 Windows 网络盘。
 */

use super::*;

#[test]
fn host_match_is_exact_and_case_insensitive() {
    assert!(remote_matches_host(
        "\\\\192.168.88.186\\Video",
        "192.168.88.186"
    ));
    assert!(remote_matches_host("\\\\NAS\\Video", "nas"));
    assert!(!remote_matches_host(
        "\\\\192.168.88.1860\\Video",
        "192.168.88.186"
    ));
    assert!(!remote_matches_host(
        "\\\\192.168.88.18\\Video",
        "192.168.88.186"
    ));
}

#[test]
fn connected_and_remembered_duplicates_are_removed_once() {
    let item = NetworkResource {
        local_name: Some("X:".to_string()),
        remote_name: "\\\\192.168.88.186\\personal_folder".to_string(),
    };
    let mut resources = vec![item.clone(), item];
    deduplicate(&mut resources);
    assert_eq!(resources.len(), 1);
}
