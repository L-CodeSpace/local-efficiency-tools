#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if tauri_app_lib::run_helper_mode_if_requested() {
        return;
    }
    tauri_app_lib::run();
}
