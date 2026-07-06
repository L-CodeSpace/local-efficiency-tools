/*
 * 核心职责：定义 hosts 管理模块 DTO。
 * 业务痛点：hosts 条目、变更计划和 helper 状态需要稳定传递给前端。
 * 能力边界：只描述 hosts 管理契约，不读写系统 hosts 文件。
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub raw: String,
    pub ip: Option<String>,
    pub hosts: Vec<String>,
    pub enabled: bool,
    pub is_comment_or_blank: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsChangeRequest {
    pub action: HostsChangeAction,
    pub ip: Option<String>,
    pub host: String,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostsChangeAction {
    Add,
    Remove,
    Toggle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsChangePlan {
    pub id: String,
    pub summary: String,
    pub line_count: usize,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsHelperStatus {
    pub required: bool,
    pub installed: bool,
    pub running: bool,
    pub token_exists: bool,
    pub needs_repair: bool,
    pub service_name: Option<String>,
    pub platform: String,
    pub helper_kind: Option<String>,
    pub install_supported: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct StoredHostsChangePlan {
    pub plan: HostsChangePlan,
    pub request: HostsChangeRequest,
}
