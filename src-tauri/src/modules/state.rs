/*
 * 核心职责：定义应用运行期共享状态。
 * 业务痛点：任务、授权根、计划缓存和后台进程需要跨模块共享且受锁保护。
 * 能力边界：只保存内存状态，不实现业务操作。
 */

use std::{
    collections::HashMap,
    path::PathBuf,
    process::Child,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

use crate::modules::{
    file_ops::dto::{AuthorizedRoot, StoredFileOperationPlan},
    hosts::dto::StoredHostsChangePlan,
    jobs::dto::JobSnapshot,
    media::dto::StoredMediaPlan,
    rename::dto::RenamePlan,
};

#[derive(Clone, Default)]
pub struct AppState {
    pub authorized_roots: Arc<Mutex<HashMap<String, AuthorizedRoot>>>,
    pub jobs: Arc<Mutex<HashMap<String, JobSnapshot>>>,
    pub file_plans: Arc<Mutex<HashMap<String, StoredFileOperationPlan>>>,
    pub rename_plans: Arc<Mutex<HashMap<String, RenamePlan>>>,
    pub hosts_plans: Arc<Mutex<HashMap<String, StoredHostsChangePlan>>>,
    pub media_plans: Arc<Mutex<HashMap<String, StoredMediaPlan>>>,
    pub job_processes: Arc<Mutex<HashMap<String, Vec<Child>>>>,
    pub mount_sessions: Arc<Mutex<HashMap<String, MountSession>>>,
    pub shutdown_started: Arc<AtomicBool>,
}

pub enum MountSession {
    NativeSmb {
        workspace_id: String,
        workspace_name: String,
        mounts: Vec<NativeSmbMount>,
    },
    FtpCombine {
        child: Child,
        workspace_id: String,
        workspace_name: String,
        target: PathBuf,
        display_target: Option<PathBuf>,
        rc_addr: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct NativeSmbMount {
    pub remote: String,
    pub target: PathBuf,
    pub display_target: Option<PathBuf>,
    pub marker_path: Option<PathBuf>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
