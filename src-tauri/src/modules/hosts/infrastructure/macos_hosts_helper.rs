#![cfg(target_os = "macos")]

/*
 * 核心职责：macOS privileged hosts helper 基础设施入口。
 * 业务痛点：macOS 写 hosts 默认每次都要管理员授权，需要一次安装的受限后台服务。
 * 能力边界：只装配 LaunchDaemon、Unix socket 和固定 hosts 写入能力。
 */

use std::{
    fs,
    io::{self, ErrorKind, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    modules::hosts::{dto::HostsHelperStatus, infrastructure::hosts_file},
    shared::error::{AppError, AppResult},
};

#[path = "macos_hosts_helper/config.rs"]
mod config;
#[path = "macos_hosts_helper/daemon.rs"]
mod daemon;
#[path = "macos_hosts_helper/install.rs"]
mod install;
#[path = "macos_hosts_helper/pipe.rs"]
mod pipe;
#[path = "macos_hosts_helper/protocol.rs"]
mod protocol;
#[path = "macos_hosts_helper/status.rs"]
mod status;
#[path = "macos_hosts_helper/util.rs"]
mod util;

use config::*;
use daemon::*;
use install::*;
use pipe::*;
use protocol::*;
pub use protocol::{run_cli_if_requested, SERVICE_NAME};
pub use status::{helper_status, install_helper, repair_helper, uninstall_helper, write_hosts};
use util::*;
