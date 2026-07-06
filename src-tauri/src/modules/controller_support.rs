/*
 * 核心职责：转换 Tauri join error。
 * 业务痛点：异步阻塞任务错误必须转成统一 AppError。
 * 能力边界：只处理 IPC 层错误适配。
 */

use crate::shared::error::AppError;

pub(crate) fn join_error(error: tauri::Error) -> AppError {
    AppError::new("background_task_failed", "后台任务执行失败").with_detail(error.to_string())
}
