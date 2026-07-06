/*
 * 核心职责：定义批量重命名模块 DTO。
 * 业务痛点：预览计划、执行请求和冲突状态需要独立于通用文件操作契约维护。
 * 能力边界：只描述批量重命名 IPC 契约，不执行文件系统操作。
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePreviewRequest {
    pub root: String,
    pub pattern: String,
    pub replacement: String,
    pub max_depth: usize,
    pub preserve_extension: bool,
    pub use_regex: Option<bool>,
    pub auto_resolve_collision: Option<bool>,
    pub collision_start_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePreviewItem {
    pub original_path: String,
    pub original_name: String,
    pub new_name: String,
    pub selected: bool,
    pub collision: bool,
    pub auto_resolved: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlan {
    pub id: String,
    pub root: String,
    pub items: Vec<RenamePreviewItem>,
    pub confirmation_token: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameExecuteRequest {
    pub plan_id: String,
    pub confirmation_token: String,
    pub selected_original_paths: Option<Vec<String>>,
}
