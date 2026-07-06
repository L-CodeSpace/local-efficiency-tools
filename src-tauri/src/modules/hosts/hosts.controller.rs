/*
 * 核心职责：暴露 hosts 管理命令。
 * 业务痛点：hosts 修改需要 helper 状态和确认计划共同约束。
 * 能力边界：只承接 hosts 应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        controller_support::join_error,
        hosts::{
            dto::{HostEntry, HostsChangePlan, HostsChangeRequest, HostsHelperStatus},
            service,
        },
        state::AppState,
    },
    shared::error::AppError,
};

/// 读取 hosts 记录。
///
/// 参数约束：读取系统 hosts 文件，不接收前端路径。
/// 返回含义：返回解析后的 hosts 条目列表。
#[tauri::command]
pub fn hosts_read() -> Result<Vec<HostEntry>, AppError> {
    service::read_hosts()
}

/// 获取 hosts 文件路径。
///
/// 参数约束：由后端按当前平台决定路径，不接收前端参数。
/// 返回含义：返回当前系统 hosts 文件路径。
#[tauri::command]
pub fn hosts_get_path() -> String {
    service::hosts_path()
}

/// 获取 hosts 写入辅助状态。
///
/// 参数约束：由后端按当前平台探测，不接收前端参数。
/// 返回含义：返回是否需要高权限辅助以及当前辅助状态。
#[tauri::command]
pub fn hosts_get_status(app: AppHandle) -> Result<HostsHelperStatus, AppError> {
    service::helper_status(&app)
}

/// 安装 hosts helper。
///
/// 参数约束：只安装当前平台受支持的受限 hosts helper；安装过程会触发一次系统授权。
/// 返回含义：返回安装后的 helper 状态。
#[tauri::command]
pub async fn hosts_install_helper(app: AppHandle) -> Result<HostsHelperStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || service::install_helper(&app))
        .await
        .map_err(join_error)?
}

/// 修复 hosts helper。
///
/// 参数约束：只更新本应用注册的 hosts helper 路径与 token 配置；修复过程会触发一次系统授权。
/// 返回含义：返回修复后的 helper 状态。
#[tauri::command]
pub async fn hosts_repair_helper(app: AppHandle) -> Result<HostsHelperStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || service::repair_helper(&app))
        .await
        .map_err(join_error)?
}

/// 卸载 hosts helper。
///
/// 参数约束：只卸载本应用注册的 hosts helper；卸载过程会触发一次系统授权。
/// 返回含义：返回卸载后的 helper 状态。
#[tauri::command]
pub async fn hosts_uninstall_helper(app: AppHandle) -> Result<HostsHelperStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || service::uninstall_helper(&app))
        .await
        .map_err(join_error)?
}

/// 预览 hosts 变更。
///
/// 参数约束：`request` 只允许 add、remove、toggle 三类 hosts 变更。
/// 返回含义：返回 hosts 变更计划和确认 token。
#[tauri::command]
pub fn hosts_preview_change(
    state: State<'_, AppState>,
    request: HostsChangeRequest,
) -> Result<HostsChangePlan, AppError> {
    service::preview_change(state.inner(), request)
}

/// 执行 hosts 变更。
///
/// 参数约束：`planId` 和 `confirmationToken` 必须来自有效 hosts 变更计划。
/// 返回含义：返回写入后的 hosts 条目列表。
#[tauri::command]
pub fn hosts_execute_change(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    confirmation_token: String,
) -> Result<Vec<HostEntry>, AppError> {
    service::execute_change(app, state.inner(), plan_id, confirmation_token)
}
