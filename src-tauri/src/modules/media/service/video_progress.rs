/*
 * 核心职责：维护视频任务聚合进度。
 * 业务痛点：多目标输出的进度统计容易错算，需要独立封装。
 * 能力边界：只计算和更新视频任务进度状态。
 */

use super::*;

pub(super) fn group_video_work(work: Vec<MediaWorkItem>) -> Vec<VideoWorkGroup> {
    let mut groups: Vec<VideoWorkGroup> = Vec::new();
    for (index, item) in work.into_iter().enumerate() {
        if let Some(group) = groups.iter_mut().find(|group| group.input == item.input) {
            group.items.push(IndexedMediaWorkItem { index, item });
        } else {
            groups.push(VideoWorkGroup {
                id: display_path(&item.input),
                input: item.input.clone(),
                items: vec![IndexedMediaWorkItem { index, item }],
            });
        }
    }
    groups
}

pub(super) fn update_video_progress_item(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    item_id: &str,
    update: impl FnOnce(&mut JobProgressItem),
) -> AppResult<JobSnapshot> {
    update_job(app, state, job_id, |snapshot| {
        if let Some(item) = snapshot
            .progress_items
            .iter_mut()
            .find(|item| item.id == item_id)
        {
            update(item);
        }
        snapshot.progress = aggregate_progress_from_items(&snapshot.progress_items);
        snapshot.message = video_job_message(&snapshot.progress_items);
    })
}

pub(super) fn mark_video_target_completed(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    item_id: &str,
    target_index: usize,
    target_count: usize,
    target_label: &str,
    artifact: Option<String>,
) {
    let _ = update_video_progress_item(app, state, job_id, item_id, |item| {
        item.completed_targets = item.completed_targets.max(target_index + 1);
        item.progress = video_item_progress(target_index + 1, target_count, 0.0);
        item.current_target = Some(target_label.to_string());
        item.frame = item.total_frames;
        item.message = format!("{} 已完成。", target_label);
        if let Some(artifact) = artifact {
            item.artifacts.push(artifact);
        }
    });
}

pub(super) fn mark_video_target_failed(
    app: &AppHandle,
    state: &AppState,
    job_id: &str,
    item_id: &str,
    target_index: usize,
    target_count: usize,
    target_label: &str,
    detail: String,
) {
    let _ = update_video_progress_item(app, state, job_id, item_id, |item| {
        item.completed_targets = item.completed_targets.max(target_index + 1);
        item.progress = video_item_progress(target_index + 1, target_count, 0.0);
        item.current_target = Some(target_label.to_string());
        item.message = format!("{} 失败。", target_label);
        item.error = Some(match item.error.take() {
            Some(existing) if !existing.is_empty() => {
                format!("{existing}\n{target_label}: {detail}")
            }
            _ => format!("{target_label}: {detail}"),
        });
    });
}

pub(super) fn aggregate_progress_from_items(items: &[JobProgressItem]) -> u8 {
    if items.is_empty() {
        return 0;
    }
    let total = items
        .iter()
        .map(|item| item.progress as usize)
        .sum::<usize>();
    (total / items.len()).min(100) as u8
}

pub(super) fn video_job_message(items: &[JobProgressItem]) -> String {
    let total = items.len();
    let completed = items
        .iter()
        .filter(|item| {
            matches!(
                item.status,
                JobProgressItemStatus::Succeeded | JobProgressItemStatus::Failed
            )
        })
        .count();
    if let Some(running) = items
        .iter()
        .find(|item| item.status == JobProgressItemStatus::Running)
    {
        if let Some(target) = &running.current_target {
            return format!(
                "正在处理 {}/{}：{} -> {}",
                completed + 1,
                total,
                running.label,
                target
            );
        }
        return format!("正在处理 {}/{}：{}", completed + 1, total, running.label);
    }
    format!("视频处理进度：{}/{}。", completed, total)
}

pub(super) fn video_item_progress(
    completed_targets: usize,
    target_count: usize,
    target_fraction: f64,
) -> u8 {
    let target_count = target_count.max(1);
    let progress = ((completed_targets as f64 + target_fraction.clamp(0.0, 0.995))
        / target_count as f64)
        * 100.0;
    progress.floor().clamp(0.0, 100.0) as u8
}
