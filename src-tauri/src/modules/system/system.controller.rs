/*
 * 核心职责：暴露系统信息命令。
 * 业务痛点：系统概览和硬件信息需要保持 command 名稳定。
 * 能力边界：只承接 system 应用服务。
 */

use tauri::AppHandle;

use crate::{
    modules::system::{
        dto::{HardwareInfo, SystemOverview},
        service,
    },
    shared::error::AppError,
};

/// 获取系统运行概览。
///
/// 参数约束：由后端读取应用目录、当前目录和平台信息，不接收前端参数。
/// 返回含义：返回应用数据目录、当前目录、平台和运行时策略。
#[tauri::command]
pub fn system_overview(app: AppHandle) -> Result<SystemOverview, AppError> {
    service::overview(&app)
}

/// 获取硬件信息。
///
/// 参数约束：由后端读取当前设备硬件信息，不接收前端参数。
/// 返回含义：返回操作系统、CPU、内存、主板和显卡摘要。
#[tauri::command]
pub fn system_hardware_info() -> HardwareInfo {
    service::hardware_info()
}
