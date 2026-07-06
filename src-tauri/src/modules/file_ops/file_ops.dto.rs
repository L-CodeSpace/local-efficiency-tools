/*
 * 核心职责：定义文件管理模块 DTO。
 * 业务痛点：文件授权、目录读取和文件操作计划需要稳定的 IPC 契约。
 * 能力边界：不包含批量重命名契约，重命名已拆入 rename 模块。
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizedRoot {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub parent: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_at: Option<u64>,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLocations {
    pub current_dir: String,
    pub executable_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileOperationKind {
    WriteText,
    Delete,
    Rename,
    CreateFile,
    CreateDir,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FileOperationRequest {
    WriteText {
        path: String,
        content: String,
    },
    Delete {
        path: String,
        recursive: Option<bool>,
    },
    Rename {
        path: String,
        new_name: String,
    },
    CreateFile {
        path: String,
    },
    CreateDir {
        path: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOperationPlan {
    pub id: String,
    pub kind: FileOperationKind,
    pub target_path: String,
    pub summary: String,
    pub risk: OperationRisk,
    pub requires_confirmation: bool,
    pub confirmation_token: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct StoredFileOperationPlan {
    pub plan: FileOperationPlan,
    pub request: FileOperationRequest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRecursiveListRequest {
    pub path: String,
    pub max_depth: usize,
    pub extensions: Option<Vec<String>>,
    pub files_only: Option<bool>,
}
