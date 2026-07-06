/*
 * 核心职责：暴露媒体处理命令。
 * 业务痛点：媒体任务和运行时下载涉及阻塞任务，IPC 层只负责调度。
 * 能力边界：只承接 media 应用服务。
 */

use tauri::{AppHandle, State};

use crate::{
    modules::{
        controller_support::join_error,
        jobs::dto::JobSnapshot,
        media::{
            dto::{
                MediaPerformanceProfile, MediaPlan, MediaPlanRequest, MediaPreviewRequest,
                MediaProbeInfo, MediaRuntimeStatus,
            },
            service,
        },
        state::AppState,
    },
    shared::error::AppError,
};

/// 创建媒体处理计划。
///
/// 参数约束：`request.inputs` 必须来自用户选择或授权目录预览。
/// 返回含义：返回媒体处理计划和确认 token。
#[tauri::command]
pub fn media_create_plan(
    state: State<'_, AppState>,
    request: MediaPlanRequest,
) -> Result<MediaPlan, AppError> {
    service::create_plan(state.inner(), request)
}

/// 启动媒体任务。
///
/// 参数约束：`planId` 和 `confirmationToken` 必须来自有效媒体计划。
/// 返回含义：返回启动后的任务快照。
#[tauri::command]
pub fn media_start_job(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: String,
    confirmation_token: String,
) -> Result<JobSnapshot, AppError> {
    service::start_job(app, state.inner().clone(), plan_id, confirmation_token)
}

/// 预览媒体输入。
///
/// 参数约束：`request.root` 必须位于已授权文件根下，按媒体类型和深度筛选。
/// 返回含义：返回符合条件的媒体文件路径列表。
#[tauri::command]
pub fn media_preview_inputs(
    app: AppHandle,
    state: State<'_, AppState>,
    request: MediaPreviewRequest,
) -> Result<Vec<String>, AppError> {
    service::preview_inputs(&app, state.inner(), request)
}

/// 探测视频详情。
///
/// 参数约束：`path` 必须位于已授权文件根下，且必须是文件。
/// 返回含义：返回 ffprobe 解析出的容器、时长、码率和流信息。
#[tauri::command]
pub fn media_probe_video(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<MediaProbeInfo, AppError> {
    service::probe_video(&app, state.inner(), path)
}

/// 获取媒体处理性能建议。
///
/// 参数约束：由后端读取本机硬件和 FFmpeg 编码器能力，不接收前端参数。
/// 返回含义：返回设备摘要、可用编码器和推荐的 FFmpeg 参数。
#[tauri::command]
pub fn media_performance_profile(app: AppHandle) -> Result<MediaPerformanceProfile, AppError> {
    service::performance_profile(&app)
}

/// 获取媒体运行时状态。
///
/// 参数约束：由后端探测 FFmpeg 状态，不接收前端参数。
/// 返回含义：返回 FFmpeg 是否可用以及版本或错误信息。
#[tauri::command]
pub fn media_runtime_status(app: AppHandle) -> Result<MediaRuntimeStatus, AppError> {
    service::runtime_status_with_app(&app)
}

/// 手动下载 FFmpeg 运行时。
///
/// 参数约束：下载源由后端按当前平台固定选择，不接收前端 URL。
/// 返回含义：下载、校验并安装后返回新的 FFmpeg 运行时状态。
#[tauri::command]
pub async fn media_download_runtime(app: AppHandle) -> Result<MediaRuntimeStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || service::download_runtime(&app))
        .await
        .map_err(join_error)?
}
