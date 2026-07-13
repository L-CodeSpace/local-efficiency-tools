/*
 * 核心职责：暴露远程连接与挂载工作区 Tauri command。
 * 业务痛点：阻塞协议探测和系统挂载必须离开 Tauri 事件线程。
 * 能力边界：只做状态注入、阻塞调度和错误边界，不实现协议细节。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        controller_support::join_error,
        mounts::{
            dto::{
                ConnectionProbeResult, MountDependencyStatus, MountRuntimeStatus, MountUiContext,
                MountWorkspace, MountWorkspaceInput, RemoteConnection, RemoteConnectionInput,
            },
            service::{runtime, workspaces},
        },
        state::AppState,
    },
    shared::error::AppError,
};

#[tauri::command]
pub async fn mounts_get_runtime_status(app: AppHandle) -> Result<MountRuntimeStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || runtime::runtime_status(&app))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_download_runtime(app: AppHandle) -> Result<MountRuntimeStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || runtime::download_runtime(&app))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub fn mounts_check_dependencies() -> MountDependencyStatus {
    runtime::check_dependencies()
}

#[tauri::command]
pub fn mounts_get_ui_context(app: AppHandle) -> Result<MountUiContext, AppError> {
    runtime::ui_context(&app)
}

#[tauri::command]
pub async fn mounts_list_connections(app: AppHandle) -> Result<Vec<RemoteConnection>, AppError> {
    tauri::async_runtime::spawn_blocking(move || workspaces::list_connections(&app))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_save_connection(
    app: AppHandle,
    input: RemoteConnectionInput,
) -> Result<RemoteConnection, AppError> {
    tauri::async_runtime::spawn_blocking(move || workspaces::save_connection(&app, input))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_delete_connection(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::delete_connection(&app, &state, &id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_probe_connection(
    app: AppHandle,
    connection_id: String,
) -> Result<ConnectionProbeResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || workspaces::probe(&app, &connection_id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_list_workspaces(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<MountWorkspace>, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::list_workspaces(&app, &state))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_create_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    input: MountWorkspaceInput,
) -> Result<MountWorkspace, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::create_workspace(&app, &state, input))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_delete_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::delete_workspace(&app, &state, &id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_set_workspace_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<MountWorkspace, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspaces::set_workspace_enabled(&app, &state, &id, enabled)
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_refresh_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    path: Option<String>,
) -> Result<MountWorkspace, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        workspaces::refresh_workspace(&app, &state, &id, path)
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_repair_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<MountWorkspace, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::repair_workspace(&app, &state, &id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn mounts_unmount_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || workspaces::unmount_all_workspaces(&app, &state))
        .await
        .map_err(join_error)?
}
