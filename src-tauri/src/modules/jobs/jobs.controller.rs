/*
 * 核心职责：暴露任务查询命令。
 * 业务痛点：后台任务状态需要统一从 AppState 读取。
 * 能力边界：只承接 jobs 应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        jobs::{dto::JobSnapshot, service},
        state::AppState,
    },
    shared::error::AppError,
};

/// 列出后台任务快照。
///
/// 参数约束：只读取后端内存中的任务状态，不触发任务执行。
/// 返回含义：返回所有任务的当前快照列表。
#[tauri::command]
pub fn jobs_list(state: State<'_, AppState>) -> Result<Vec<JobSnapshot>, AppError> {
    service::list_jobs(state.inner())
}

/// 获取指定任务快照。
///
/// 参数约束：`jobId` 必须是后端任务 ID。
/// 返回含义：返回任务快照；不存在时返回空值。
#[tauri::command]
pub fn jobs_get(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<JobSnapshot>, AppError> {
    service::get_job(state.inner(), &job_id)
}

/// 取消指定任务。
///
/// 参数约束：`jobId` 必须是后端任务 ID，已完成任务不会重新执行。
/// 返回含义：返回取消后的任务快照。
#[tauri::command]
pub fn jobs_cancel(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<JobSnapshot, AppError> {
    service::cancel_job(&app, state.inner(), &job_id)
}
