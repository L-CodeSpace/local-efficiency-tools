/*
 * 核心职责：暴露批量重命名命令。
 * 业务痛点：批量操作需要先预览后执行。
 * 能力边界：只承接 rename 应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        jobs::dto::JobSnapshot,
        rename::{
            dto::{RenameExecuteRequest, RenamePlan, RenamePreviewRequest},
            service,
        },
        state::AppState,
    },
    shared::error::AppError,
};

/// 预览批量重命名。
///
/// 参数约束：`request.root` 必须位于已授权文件根下，规则由后端解析。
/// 返回含义：返回批量重命名计划、条目预览和确认 token。
#[tauri::command]
pub fn batch_rename_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RenamePreviewRequest,
) -> Result<RenamePlan, AppError> {
    service::preview(&app, state.inner(), request)
}

/// 执行批量重命名。
///
/// 参数约束：`request` 必须引用有效预览计划，可携带用户勾选的原始路径。
/// 返回含义：返回代表本次执行结果的任务快照。
#[tauri::command]
pub fn batch_rename_execute(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RenameExecuteRequest,
) -> Result<JobSnapshot, AppError> {
    service::execute(app, state.inner().clone(), request)
}
