/*
 * 核心职责：集中声明 Tauri command 白名单。
 * 业务痛点：SDK 生成器需要稳定扫描 generate_handler，但启动层不应承载命令清单细节。
 * 能力边界：只聚合 controller command，不处理插件、托盘或生命周期。
 */

use crate::modules::{
    app::controller::*, file_ops::controller::*, hosts::controller::*, jobs::controller::*,
    media::controller::*, mounts::controller::*, rename::controller::*, system::controller::*,
};

pub fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        system_overview,
        system_hardware_info,
        jobs_list,
        jobs_get,
        jobs_cancel,
        file_get_locations,
        file_list_roots,
        file_authorize_path,
        file_list_dir,
        file_list_dir_recursive,
        file_read_text,
        file_preview_operation,
        file_execute_operation,
        batch_rename_preview,
        batch_rename_execute,
        media_create_plan,
        media_start_job,
        media_preview_inputs,
        media_probe_video,
        media_performance_profile,
        media_runtime_status,
        media_download_runtime,
        hosts_read,
        hosts_get_path,
        hosts_get_status,
        hosts_install_helper,
        hosts_repair_helper,
        hosts_uninstall_helper,
        hosts_preview_change,
        hosts_execute_change,
        mounts_get_runtime_status,
        mounts_download_runtime,
        mounts_check_dependencies,
        mounts_get_ui_context,
        mounts_list_connections,
        mounts_save_connection,
        mounts_delete_connection,
        mounts_probe_connection,
        mounts_list_workspaces,
        mounts_create_workspace,
        mounts_delete_workspace,
        mounts_set_workspace_enabled,
        mounts_refresh_workspace,
        mounts_repair_workspace,
        mounts_unmount_all,
        app_settings_get_background,
        app_settings_set_background,
    ]
}
