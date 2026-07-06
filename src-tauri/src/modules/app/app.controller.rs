/*
 * 核心职责：暴露应用设置命令。
 * 业务痛点：后台运行设置属于挂载运行策略的一部分。
 * 能力边界：只承接设置读写入口。
 */

use tauri::AppHandle;

use crate::{
    modules::mounts::{dto::BackgroundSettings, service::profiles},
    shared::error::AppError,
};

/// 获取后台运行设置。
///
/// 参数约束：设置由后端从应用数据目录读取，不接收前端路径。
/// 返回含义：返回关闭窗口时是否允许隐藏到托盘。
#[tauri::command]
pub fn app_settings_get_background(app: AppHandle) -> Result<BackgroundSettings, AppError> {
    profiles::get_background_settings(&app)
}

/// 更新后台运行设置。
///
/// 参数约束：`enabled` 表示关闭窗口时是否允许隐藏到托盘。
/// 返回含义：返回保存后的后台运行设置。
#[tauri::command]
pub fn app_settings_set_background(
    app: AppHandle,
    enabled: bool,
) -> Result<BackgroundSettings, AppError> {
    profiles::set_background_enabled(&app, enabled)
}
