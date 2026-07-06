#![cfg(target_os = "windows")]

/*
 * 核心职责：Windows hosts helper 基础设施入口。
 * 业务痛点：对外模块路径必须稳定，拆分实现不能影响现有调用方。
 * 能力边界：只装配同模块实现分片，不承载具体业务流程。
 */

use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, ErrorKind},
    os::windows::{ffi::OsStrExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED,
        ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST, ERROR_SERVICE_NOT_ACTIVE,
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    },
    Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    },
    System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, WaitNamedPipeW,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
        PIPE_WAIT,
    },
};

use crate::{
    modules::hosts::{dto::HostsHelperStatus, infrastructure::hosts_file},
    shared::error::{AppError, AppResult},
};

#[path = "windows_hosts_helper/config.rs"]
mod config;
#[path = "windows_hosts_helper/install.rs"]
mod install;
#[path = "windows_hosts_helper/pipe.rs"]
mod pipe;
#[path = "windows_hosts_helper/protocol.rs"]
mod protocol;
#[path = "windows_hosts_helper/security.rs"]
mod security;
#[path = "windows_hosts_helper/service.rs"]
mod service;
#[path = "windows_hosts_helper/status.rs"]
mod status;
#[path = "windows_hosts_helper/util.rs"]
mod util;

#[cfg(test)]
#[path = "windows_hosts_helper/tests.rs"]
mod tests;

use config::*;
use install::*;
use pipe::*;
use protocol::*;
pub use protocol::{run_cli_if_requested, SERVICE_NAME};
use security::*;
use service::*;
pub use status::{helper_status, install_helper, repair_helper, uninstall_helper, write_hosts};
use util::*;
