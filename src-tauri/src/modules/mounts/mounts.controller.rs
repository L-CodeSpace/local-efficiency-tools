/*
 * 核心职责：暴露远程挂载命令。
 * 业务痛点：rclone 操作多为阻塞流程，IPC 层必须放入阻塞线程。
 * 能力边界：只承接 mounts 应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        controller_support::join_error,
        mounts::{
            dto::{
                MountDependencyStatus, MountProfile, MountProfileInput, MountProfileLog,
                MountRuntimeStatus, MountTestResult, MountUiContext,
            },
            service::{logs, profiles, runtime},
        },
        state::AppState,
    },
    shared::error::AppError,
};

/// 获取 rclone 运行时状态。
///
/// 参数约束：rclone 路径由后端固定到应用数据目录，不接收前端路径。
/// 返回含义：返回本地 rclone 是否已安装、版本、路径和是否需要下载。
#[tauri::command]
pub async fn mounts_get_runtime_status(app: AppHandle) -> Result<MountRuntimeStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || runtime::runtime_status(&app))
        .await
        .map_err(join_error)?
}

/// 手动下载 rclone 运行时。
///
/// 参数约束：下载源由后端按当前平台固定选择，不接收前端 URL。
/// 返回含义：下载、校验并安装后返回新的 rclone 运行时状态。
#[tauri::command]
pub async fn mounts_download_runtime(app: AppHandle) -> Result<MountRuntimeStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || runtime::download_runtime(&app))
        .await
        .map_err(join_error)?
}

/// 检查 rclone mount 系统依赖。
///
/// 参数约束：当前只检测 Windows WinFsp 与 macOS macFUSE。
/// 返回含义：返回依赖是否就绪及官方安装地址。
#[tauri::command]
pub fn mounts_check_dependencies() -> MountDependencyStatus {
    runtime::check_dependencies()
}

/// 获取远程挂载页面展示上下文。
///
/// 参数约束：平台与默认挂载目录由后端运行环境决定，不接收前端路径。
/// 返回含义：返回当前平台、默认挂载根目录、示例路径和是否支持 Windows 盘符。
#[tauri::command]
pub fn mounts_get_ui_context(app: AppHandle) -> Result<MountUiContext, AppError> {
    runtime::ui_context(&app)
}

/// 列出远程挂载配置。
///
/// 参数约束：配置由后端从应用数据目录读取，不接收前端路径。
/// 返回含义：返回 Profile 列表，并附带当前运行时挂载状态。
#[tauri::command]
pub async fn mounts_list_profiles(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<MountProfile>, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || profiles::list_profiles(&app, &state))
        .await
        .map_err(join_error)?
}

/// 保存远程挂载配置。
///
/// 参数约束：`input.protocol` 仅支持 ftp、sftp、webdav；密码留空不覆盖已有凭据。
/// 返回含义：返回保存后的配置；若启用挂载，会返回当前挂载状态。
#[tauri::command]
pub async fn mounts_save_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    input: MountProfileInput,
) -> Result<MountProfile, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || profiles::save_profile(app, state, input))
        .await
        .map_err(join_error)?
}

/// 删除远程挂载配置。
///
/// 参数约束：`id` 必须是已存在的 Profile ID。
/// 返回含义：成功后会先卸载运行中的挂载，再删除配置和 rclone remote。
#[tauri::command]
pub async fn mounts_delete_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || profiles::delete_profile(app, state, id))
        .await
        .map_err(join_error)?
}

/// 启用或停用远程挂载。
///
/// 参数约束：启用前要求 rclone runtime 和系统挂载依赖就绪。
/// 返回含义：返回更新后的 Profile 及当前挂载状态。
#[tauri::command]
pub async fn mounts_set_profile_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<MountProfile, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        profiles::set_profile_enabled(app, state, id, enabled)
    })
    .await
    .map_err(join_error)?
}

/// 测试远程挂载连接。
///
/// 参数约束：`id` 必须是已存在的 Profile ID，仅测试远程可访问性。
/// 返回含义：返回测试是否成功和 rclone 输出摘要。
#[tauri::command]
pub async fn mounts_test_profile(app: AppHandle, id: String) -> Result<MountTestResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || profiles::test_profile(app, id))
        .await
        .map_err(join_error)?
}

/// 读取指定 profile 的 rclone 日志。
///
/// 参数约束：`id` 必须是已存在的 Profile ID；日志路径由后端固定计算。
/// 返回含义：返回日志文件路径、元数据和尾部日志内容。
#[tauri::command]
pub async fn mounts_get_profile_log(
    app: AppHandle,
    id: String,
    max_lines: Option<usize>,
) -> Result<MountProfileLog, AppError> {
    tauri::async_runtime::spawn_blocking(move || logs::get_profile_log(app, id, max_lines))
        .await
        .map_err(join_error)?
}

/// 卸载全部远程挂载。
///
/// 参数约束：只影响本应用启动并追踪的 rclone mount 进程。
/// 返回含义：成功表示所有已知挂载进程已请求退出，配置启用状态已关闭。
#[tauri::command]
pub async fn mounts_unmount_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || profiles::unmount_all(app, state))
        .await
        .map_err(join_error)?
}
