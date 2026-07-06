/*
 * 核心职责：管理后台任务状态。
 * 业务痛点：任务创建、查询、取消和进度广播必须集中处理，避免各业务模块重复维护状态机。
 * 能力边界：不直接执行具体业务任务，只维护通用任务快照和进度事件。
 */

use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{
    modules::{
        jobs::dto::{JobProgressItemStatus, JobResult, JobSnapshot, JobStatus},
        state::AppState,
    },
    shared::error::{AppError, AppResult},
};

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn create_job(
    state: &AppState,
    kind: impl Into<String>,
    title: impl Into<String>,
) -> AppResult<JobSnapshot> {
    let now = now_millis();
    let job = JobSnapshot {
        id: Uuid::new_v4().to_string(),
        kind: kind.into(),
        title: title.into(),
        status: JobStatus::Queued,
        progress: 0,
        message: "任务已创建，等待执行。".to_string(),
        created_at: now,
        updated_at: now,
        progress_items: Vec::new(),
        result: None,
        error: None,
    };
    state
        .jobs
        .lock()
        .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?
        .insert(job.id.clone(), job.clone());
    Ok(job)
}

pub fn list_jobs(state: &AppState) -> AppResult<Vec<JobSnapshot>> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?
        .values()
        .cloned()
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(jobs)
}

pub fn get_job(state: &AppState, job_id: &str) -> AppResult<Option<JobSnapshot>> {
    Ok(state
        .jobs
        .lock()
        .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?
        .get(job_id)
        .cloned())
}

pub fn cancel_job(app: &AppHandle, state: &AppState, job_id: &str) -> AppResult<JobSnapshot> {
    if let Some(children) = state
        .job_processes
        .lock()
        .map_err(|_| AppError::fatal("job_process_registry_poisoned", "任务进程注册表锁已损坏"))?
        .remove(job_id)
    {
        kill_job_processes(children);
    }

    update_job(app, state, job_id, |job| {
        if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
            job.status = JobStatus::Cancelled;
            job.progress = job.progress.max(1);
            job.message = "任务已取消。".to_string();
            for item in &mut job.progress_items {
                if matches!(
                    item.status,
                    JobProgressItemStatus::Queued | JobProgressItemStatus::Running
                ) {
                    item.status = JobProgressItemStatus::Cancelled;
                    item.message = "已取消。".to_string();
                    item.current_target = None;
                }
            }
        }
    })
}

pub fn cancel_running_jobs(app: &AppHandle, state: &AppState) -> AppResult<usize> {
    let mut cancelled = Vec::new();
    {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?;
        for job in jobs.values_mut() {
            if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
                job.status = JobStatus::Cancelled;
                job.progress = job.progress.max(1);
                job.message = "程序退出，任务已取消。".to_string();
                for item in &mut job.progress_items {
                    if matches!(
                        item.status,
                        JobProgressItemStatus::Queued | JobProgressItemStatus::Running
                    ) {
                        item.status = JobProgressItemStatus::Cancelled;
                        item.message = "程序退出，已取消。".to_string();
                        item.current_target = None;
                    }
                }
                job.updated_at = now_millis();
                cancelled.push(job.clone());
            }
        }
    }

    for job in &cancelled {
        let _ = app.emit("job://updated", job.clone());
    }

    if !cancelled.is_empty() {
        thread::sleep(Duration::from_millis(250));
    }

    let running = state
        .job_processes
        .lock()
        .map_err(|_| AppError::fatal("job_process_registry_poisoned", "任务进程注册表锁已损坏"))?
        .drain()
        .flat_map(|(_, children)| children)
        .collect::<Vec<_>>();

    kill_job_processes(running);

    Ok(cancelled.len())
}

fn kill_job_processes(children: Vec<std::process::Child>) {
    for mut child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn update_job(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    update: impl FnOnce(&mut JobSnapshot),
) -> AppResult<JobSnapshot> {
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?;
    let job = jobs
        .get_mut(job_id)
        .ok_or_else(|| AppError::new("job_not_found", "任务不存在"))?;
    update(job);
    job.updated_at = now_millis();
    let snapshot = job.clone();
    let _ = app.emit("job://updated", snapshot.clone());
    Ok(snapshot)
}

pub fn start_synthetic_job(
    app: AppHandle,
    state: AppState,
    mut job: JobSnapshot,
    completion_message: impl Into<String> + Send + 'static,
) -> AppResult<JobSnapshot> {
    job.status = JobStatus::Running;
    job.message = "任务开始执行。".to_string();
    job.updated_at = now_millis();
    state
        .jobs
        .lock()
        .map_err(|_| AppError::fatal("job_registry_poisoned", "任务注册表锁已损坏"))?
        .insert(job.id.clone(), job.clone());
    let _ = app.emit("job://updated", job.clone());

    let job_id = job.id.clone();
    let completion_message = completion_message.into();
    tauri::async_runtime::spawn(async move {
        for progress in [15u8, 35, 60, 85] {
            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
            let _ = update_job(&app, &state, &job_id, |snapshot| {
                if snapshot.status == JobStatus::Running {
                    snapshot.progress = progress;
                    snapshot.message = format!("执行进度 {}%。", progress);
                }
            });
        }
        let _ = update_job(&app, &state, &job_id, |snapshot| {
            if snapshot.status == JobStatus::Running {
                snapshot.status = JobStatus::Succeeded;
                snapshot.progress = 100;
                snapshot.message = completion_message;
                snapshot.result = Some(JobResult {
                    executor: "plannedExecutor".to_string(),
                    completed: true,
                    artifacts: Vec::new(),
                });
            }
        });
    });

    Ok(job)
}
