/*
 * 核心职责：暴露文件管理命令。
 * 业务痛点：文件系统访问必须通过授权边界进入应用层。
 * 能力边界：只承接文件管理应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        file_ops::{
            dto::{
                AuthorizedRoot, FileEntry, FileLocations, FileOperationPlan, FileOperationRequest,
                FileRecursiveListRequest,
            },
            service,
        },
        state::AppState,
    },
    shared::error::AppError,
};

/// 获取常用文件位置。
///
/// 参数约束：由后端读取当前目录和可执行文件目录，不接收前端路径。
/// 返回含义：返回当前工作目录和程序目录。
#[tauri::command]
pub fn file_get_locations() -> Result<FileLocations, AppError> {
    service::locations()
}

/// 列出已授权文件根。
///
/// 参数约束：包含静态安全根和用户通过选择器授权的动态根。
/// 返回含义：返回当前可访问的根目录列表。
#[tauri::command]
pub fn file_list_roots(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<AuthorizedRoot>, AppError> {
    service::list_roots(&app, state.inner())
}

/// 授权一个文件系统路径。
///
/// 参数约束：`path` 来自系统选择器；文件路径会授权其父目录。
/// 返回含义：返回新增或更新后的授权根。
#[tauri::command]
pub fn file_authorize_path(
    state: State<'_, AppState>,
    path: String,
    label: Option<String>,
) -> Result<AuthorizedRoot, AppError> {
    service::authorize_path(state.inner(), path, label)
}

/// 列出目录内容。
///
/// 参数约束：`path` 必须位于已授权文件根下。
/// 返回含义：返回该目录下的文件和子目录条目。
#[tauri::command]
pub fn file_list_dir(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<FileEntry>, AppError> {
    service::list_dir(&app, state.inner(), path)
}

/// 递归列出目录内容。
///
/// 参数约束：`request.path` 必须位于已授权文件根下，递归深度由 `maxDepth` 限制。
/// 返回含义：返回符合扩展名和文件类型条件的条目列表。
#[tauri::command]
pub fn file_list_dir_recursive(
    app: AppHandle,
    state: State<'_, AppState>,
    request: FileRecursiveListRequest,
) -> Result<Vec<FileEntry>, AppError> {
    service::list_dir_recursive(&app, state.inner(), request)
}

/// 读取文本文件。
///
/// 参数约束：`path` 必须位于已授权文件根下，且文件大小受后端限制。
/// 返回含义：返回文本文件内容。
#[tauri::command]
pub fn file_read_text(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, AppError> {
    service::read_text(&app, state.inner(), path)
}

/// 预览文件操作。
///
/// 参数约束：`request` 描述写入、删除、重命名或创建操作，目标路径必须已授权。
/// 返回含义：返回需要确认的文件操作计划和确认 token。
#[tauri::command]
pub fn file_preview_operation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: FileOperationRequest,
) -> Result<FileOperationPlan, AppError> {
    service::preview_operation(&app, state.inner(), request)
}

/// 执行文件操作。
///
/// 参数约束：`planId` 和 `confirmationToken` 必须来自有效的预览计划。
/// 返回含义：返回执行后相关文件条目；无相关条目时返回空值。
#[tauri::command]
pub fn file_execute_operation(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    confirmation_token: String,
) -> Result<Option<FileEntry>, AppError> {
    service::execute_operation(&app, state.inner(), plan_id, confirmation_token)
}
