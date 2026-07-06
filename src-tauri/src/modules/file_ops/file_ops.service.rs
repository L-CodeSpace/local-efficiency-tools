/*
 * 核心职责：文件操作应用服务入口。
 * 业务痛点：对外模块路径必须稳定，拆分实现不能影响现有调用方。
 * 能力边界：只装配同模块实现分片，不承载具体业务流程。
 */

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use tauri::AppHandle;
use uuid::Uuid;

use crate::{
    modules::{
        file_ops::dto::{
            AuthorizedRoot, FileEntry, FileLocations, FileOperationKind, FileOperationPlan,
            FileOperationRequest, FileRecursiveListRequest, OperationRisk, StoredFileOperationPlan,
        },
        jobs::service::now_millis,
        state::AppState,
    },
    shared::{
        error::{AppError, AppResult},
        fs_guard,
    },
};

#[path = "service/operations.rs"]
mod operations;
#[path = "service/roots.rs"]
mod roots;
#[path = "service/scan.rs"]
mod scan;

pub use operations::{execute_operation, preview_operation};
pub(crate) use roots::ensure_allowed_path;
pub use roots::{authorize_path, list_dir, list_dir_recursive, list_roots, locations, read_text};
use scan::*;
