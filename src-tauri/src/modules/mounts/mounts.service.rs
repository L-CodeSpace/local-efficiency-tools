/*
 * 核心职责：远程挂载应用服务入口。
 * 业务痛点：按职责拆分 rclone 运行时、配置和进程管理，避免 include! 拼接作用域。
 * 能力边界：只声明 service 子模块和共享内部依赖，不做对外 re-export。
 */

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    modules::{
        mounts::dto::{
            BackgroundSettings, ConnectionProbeResult, EffectiveTransport, MountAdvancedOptions,
            MountDependencyStatus, MountPlatform, MountRuntimeStatus, MountStatus, MountStore,
            MountUiContext, MountWorkspace, MountWorkspaceInput, ProbeShareEntry, RemoteBinding,
            RemoteConnection, RemoteConnectionInput, SmbHostCleanupResult, SmbMappingCleanupItem,
            TransportPreference, TransportProbeResult,
        },
        state::AppState,
    },
    observability,
    shared::error::{AppError, AppResult},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[path = "service/advanced_options.rs"]
mod advanced_options;
#[path = "service/dependencies.rs"]
mod dependencies;
#[path = "service/normalize.rs"]
mod normalize;
#[path = "service/runtime.rs"]
pub mod runtime;
#[path = "service/runtime_download.rs"]
mod runtime_download;
#[path = "service/storage.rs"]
mod storage;

#[path = "service/connection_probe.rs"]
mod connection_probe;
#[path = "service/ftp_combine.rs"]
mod ftp_combine;
#[path = "service/native_smb.rs"]
mod native_smb;
#[path = "service/v2_storage.rs"]
mod v2_storage;
#[path = "service/workspaces.rs"]
pub mod workspaces;
