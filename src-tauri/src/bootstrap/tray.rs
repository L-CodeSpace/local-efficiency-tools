/*
 * 核心职责：装配系统托盘和托盘菜单事件。
 * 业务痛点：后台挂载需要托盘入口，但托盘不应污染 Tauri 启动装配。
 * 能力边界：只处理托盘 UI 事件，不注册 IPC command。
 */

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use crate::{
    modules::{mounts::service::profiles as mount_profiles, shutdown, state::AppState},
    observability,
};

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let unmount_all = MenuItem::with_id(app, "unmount_all", "卸载全部挂载", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &unmount_all, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("本地效率工具")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "unmount_all" => {
                let state = app.state::<AppState>().inner().clone();
                match mount_profiles::unmount_all(app.clone(), state) {
                    Ok(()) => observability::emit_info(app, "已卸载全部 rclone 挂载。"),
                    Err(error) => {
                        observability::emit_info(app, format!("卸载 rclone 挂载失败: {}", error));
                    }
                }
            }
            "quit" => {
                let state = app.state::<AppState>().inner().clone();
                shutdown::cleanup_before_exit(app, &state);
                app.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // 【合理吞噬】托盘菜单只尝试恢复窗口焦点，失败不应中断托盘事件处理。
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
