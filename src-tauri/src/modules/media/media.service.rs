/*
 * 核心职责：媒体处理应用服务入口。
 * 业务痛点：对外模块路径必须稳定，拆分实现不能影响现有调用方。
 * 能力边界：只装配同模块实现分片，不承载具体业务流程。
 */

use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use zip::ZipArchive;

use crate::{
    modules::{
        file_ops::service::ensure_allowed_path,
        jobs::{
            dto::{JobProgressItem, JobProgressItemStatus, JobResult, JobSnapshot, JobStatus},
            service::{create_job, get_job, update_job},
        },
        media::dto::{
            ImageOutputFormat, MediaDeviceSummary, MediaEncoderCapability, MediaGpuSummary,
            MediaJobKind, MediaPerformanceProfile, MediaPlan, MediaPlanRequest,
            MediaPreviewRequest, MediaProbeInfo, MediaProbeStream, MediaRecommendedSettings,
            MediaRuntimeStatus, StoredMediaPlan, VideoAv1Encoder, VideoTarget,
        },
        state::AppState,
    },
    observability,
    shared::error::{AppError, AppResult},
};

#[path = "service/collect.rs"]
mod collect;
#[path = "service/download.rs"]
mod download;
#[path = "service/execute_linear.rs"]
mod execute_linear;
#[path = "service/execute_video.rs"]
mod execute_video;
#[path = "service/execute_video_group.rs"]
mod execute_video_group;
#[path = "service/ffmpeg_args.rs"]
mod ffmpeg_args;
#[path = "service/ffmpeg_process.rs"]
mod ffmpeg_process;
#[path = "service/finish.rs"]
mod finish;
#[path = "service/paths.rs"]
mod paths;
#[path = "service/performance.rs"]
mod performance;
#[path = "service/plan.rs"]
mod plan;
#[path = "service/probe.rs"]
mod probe;
#[path = "service/runtime.rs"]
mod runtime;
#[path = "service/runtime_status.rs"]
mod runtime_status;
#[path = "service/types.rs"]
mod types;
#[path = "service/video_probe.rs"]
mod video_probe;
#[path = "service/video_progress.rs"]
mod video_progress;
#[path = "service/work.rs"]
mod work;

#[cfg(test)]
#[path = "service/tests.rs"]
mod tests;

use collect::*;
use download::*;
use execute_linear::*;
use execute_video::*;
use execute_video_group::*;
use ffmpeg_args::*;
use ffmpeg_process::*;
use finish::*;
use paths::*;
pub use performance::performance_profile;
pub use plan::{create_plan, preview_inputs, start_job};
#[cfg(test)]
pub(crate) use probe::parse_ffprobe_output;
pub use probe::probe_video;
use runtime::*;
pub use runtime_status::{download_runtime, runtime_status, runtime_status_with_app};
use types::*;
use video_probe::*;
use video_progress::*;
use work::*;
