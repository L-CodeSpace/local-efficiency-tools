/*
 * 核心职责：聚合连接、工作区和运行会话应用服务。
 * 业务痛点：双通道挂载需要稳定入口，但连接 CRUD、业务校验和 supervisor 必须分离。
 * 能力边界：只声明子模块并导出 controller 和生命周期需要的用例。
 */

use super::*;
use std::sync::{Mutex, OnceLock};

#[path = "workspaces/connections.rs"]
mod connections;
#[path = "workspaces/lifecycle.rs"]
mod lifecycle;
#[path = "workspaces/operations.rs"]
mod operations;
#[path = "workspaces/validation.rs"]
mod validation;

pub use connections::{
    delete_connection, get_background_settings, list_connections, probe, save_connection,
    set_background_enabled,
};
pub use lifecycle::{restore_enabled_workspaces, stop_all_workspaces, unmount_all_workspaces};
pub use operations::{
    create_workspace, delete_workspace, list_workspaces, refresh_workspace, repair_workspace,
    set_workspace_enabled,
};

static WORKSPACE_OPERATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn operation_lock() -> AppResult<std::sync::MutexGuard<'static, ()>> {
    WORKSPACE_OPERATION_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| AppError::new("mount_operation_lock_failed", "挂载操作锁已损坏"))
}
