/*
 * 核心职责：装配 Tauri 应用启动流程。
 * 业务痛点：启动入口必须保持薄而稳定，避免承载命令清单、托盘和生命周期细节。
 * 能力边界：只注册全局状态、插件、启动回调和运行事件分发。
 */

mod command_registry;
mod lifecycle;
mod tray;

use tauri::Manager;

use crate::modules::state::AppState;

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                // 【合理吞噬】单实例唤醒只做窗口可见性修复，失败不影响既有主进程继续运行。
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            tray::setup(app)?;
            lifecycle::restore_background_mounts(app);
            Ok(())
        })
        .invoke_handler(command_registry::handler())
        .build(tauri::generate_context!())
        .expect("failed to build local efficiency tools")
        .run(lifecycle::handle_run_event);
}
