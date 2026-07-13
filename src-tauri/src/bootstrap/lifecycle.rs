/*
 * 核心职责：管理应用启动后的后台恢复和退出清理。
 * 业务痛点：挂载进程与任务进程必须在启动/退出生命周期内统一处理。
 * 能力边界：不注册 UI、托盘或 IPC command，只处理生命周期副作用。
 */

use tauri::Manager;

use crate::modules::{mounts::service::workspaces, shutdown, state::AppState};

const STARTUP_MOUNT_RESTORE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

pub fn restore_background_mounts(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let state = app.state::<AppState>().inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(STARTUP_MOUNT_RESTORE_DELAY);
        workspaces::restore_enabled_workspaces(app_handle, state);
    });
}

pub fn handle_run_event(app_handle: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::WindowEvent { label, event, .. } => {
            if label == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    let state = app_handle.state::<AppState>().inner().clone();
                    let app = app_handle.clone();
                    api.prevent_close();
                    tauri::async_runtime::spawn_blocking(move || {
                        shutdown::cleanup_before_exit(&app, &state);
                        app.exit(0);
                    });
                }
            }
        }
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
            let state = app_handle.state::<AppState>().inner().clone();
            shutdown::cleanup_before_exit(app_handle, &state);
        }
        _ => {}
    }
}
