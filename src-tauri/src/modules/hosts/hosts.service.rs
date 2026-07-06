/*
 * 核心职责：hosts 管理应用服务入口。
 * 业务痛点：对外模块路径必须稳定，拆分实现不能影响现有调用方。
 * 能力边界：只装配同模块实现分片，不承载具体业务流程。
 */

use std::{fs, path::Path, process::Command};

use tauri::AppHandle;
use uuid::Uuid;

use crate::{
    modules::{
        hosts::{
            dto::{
                HostEntry, HostsChangeAction, HostsChangePlan, HostsChangeRequest,
                HostsHelperStatus, StoredHostsChangePlan,
            },
            infrastructure::hosts_file,
        },
        state::AppState,
    },
    shared::error::{AppError, AppResult},
};

#[cfg(target_os = "windows")]
use crate::modules::hosts::infrastructure::windows_hosts_helper;

#[cfg(target_os = "macos")]
use crate::modules::hosts::infrastructure::macos_hosts_helper;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[path = "service/change.rs"]
mod change;
#[path = "service/public.rs"]
mod public;
#[path = "service/write.rs"]
mod write;

use change::*;
pub use public::{
    execute_change, helper_status, hosts_path, install_helper, preview_change, read_hosts,
    repair_helper, uninstall_helper,
};
use write::*;
