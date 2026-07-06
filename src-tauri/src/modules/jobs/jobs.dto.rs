/*
 * 核心职责：定义后台任务模块 DTO。
 * 业务痛点：后台任务状态、进度项和错误快照需要稳定传递给前端。
 * 能力边界：只描述任务契约，不启动或管理任务。
 */

use crate::shared::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub executor: String,
    pub completed: bool,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobProgressItemStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressItem {
    pub id: String,
    pub input_path: String,
    pub label: String,
    pub status: JobProgressItemStatus,
    pub progress: u8,
    pub message: String,
    pub current_target: Option<String>,
    pub frame: Option<u64>,
    pub total_frames: Option<u64>,
    pub completed_targets: usize,
    pub total_targets: usize,
    pub artifacts: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub status: JobStatus,
    pub progress: u8,
    pub message: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub progress_items: Vec<JobProgressItem>,
    pub result: Option<JobResult>,
    pub error: Option<AppError>,
}
