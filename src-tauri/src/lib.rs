pub mod bootstrap;
pub mod config;
pub mod modules;
pub mod observability;
pub mod platform;
pub mod shared;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    bootstrap::run();
}

#[cfg(target_os = "windows")]
pub fn run_helper_mode_if_requested() -> bool {
    modules::hosts::infrastructure::windows_hosts_helper::run_cli_if_requested()
}

#[cfg(target_os = "macos")]
pub fn run_helper_mode_if_requested() -> bool {
    modules::hosts::infrastructure::macos_hosts_helper::run_cli_if_requested()
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn run_helper_mode_if_requested() -> bool {
    false
}
