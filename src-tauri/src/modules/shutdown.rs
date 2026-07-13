/*
 * 核心职责：统一应用退出前的运行时资源清理。
 * 业务痛点：窗口关闭、托盘退出和运行时退出事件可能重复触发，必须保证清理幂等。
 * 能力边界：只协调各业务模块停止运行时资源，不删除已安装 runtime 文件。
 */

use std::sync::atomic::Ordering;

use tauri::AppHandle;

use crate::{
    modules::{jobs::service as jobs_service, mounts::service::workspaces, state::AppState},
    observability,
};

pub fn cleanup_before_exit(app: &AppHandle, state: &AppState) {
    if state.shutdown_started.swap(true, Ordering::SeqCst) {
        return;
    }

    observability::emit_info(app, "正在退出，清理运行时资源。");

    match jobs_service::cancel_running_jobs(app, state) {
        Ok(count) if count > 0 => {
            observability::emit_info(app, format!("已取消 {} 个运行中任务。", count));
        }
        Ok(_) => {}
        Err(error) => {
            observability::emit_info(app, format!("取消运行中任务失败: {}", error));
        }
    }

    workspaces::stop_all_workspaces(app, state);
    observability::emit_info(app, "运行时资源清理完成。");
}
