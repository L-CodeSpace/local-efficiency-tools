/*
 * 核心职责：远程挂载应用服务入口。
 * 业务痛点：按职责拆分 rclone 运行时、配置和进程管理，避免 include! 拼接作用域。
 * 能力边界：只声明 service 子模块和共享内部依赖，不做对外 re-export。
 */

use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
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
            BackgroundSettings, MountAdvancedOptions, MountDependencyStatus, MountPlatform,
            MountProfile, MountProfileInput, MountProfileLog, MountProtocol, MountRuntimeStatus,
            MountStatus, MountTestResult, MountUiContext,
        },
        state::{AppState, MountProcess},
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
#[path = "service/logs.rs"]
pub mod logs;
#[path = "service/normalize.rs"]
mod normalize;
#[path = "service/processes.rs"]
pub mod processes;
#[path = "service/profile_form.rs"]
mod profile_form;
#[path = "service/profiles.rs"]
pub mod profiles;
#[path = "service/rclone_config.rs"]
mod rclone_config;
#[path = "service/runtime.rs"]
pub mod runtime;
#[path = "service/runtime_download.rs"]
mod runtime_download;
#[path = "service/storage.rs"]
mod storage;
#[path = "service/target.rs"]
mod target;
#[path = "service/target_runtime.rs"]
mod target_runtime;

#[cfg(test)]
#[path = "service/advanced_options_tests.rs"]
mod advanced_options_tests;
#[cfg(test)]
#[path = "service/log_tests.rs"]
mod log_tests;
#[cfg(test)]
#[path = "service/tests.rs"]
mod tests;
