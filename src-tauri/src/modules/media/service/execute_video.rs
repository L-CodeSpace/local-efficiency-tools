/*
 * 核心职责：执行视频多目标任务。
 * 业务痛点：视频任务存在多目标、多进度和失败汇总，必须单独维护。
 * 能力边界：只处理视频任务执行主循环。
 */

use super::*;

pub(super) fn execute_video_media_job(
    app: AppHandle,
    state: AppState,
    job_id: String,
    ffmpeg_path: PathBuf,
    work: Vec<MediaWorkItem>,
) {
    let groups = group_video_work(work);
    let av1_encoders = detect_av1_encoders(&ffmpeg_path);
    let ffprobe_path = ffprobe_path_for_execution(&app).ok();
    let concurrency = video_concurrency_for_groups(&groups);

    let _ = update_job(&app, &state, &job_id, |snapshot| {
        snapshot.progress = 0;
        snapshot.progress_items = groups
            .iter()
            .map(|group| JobProgressItem {
                id: group.id.clone(),
                input_path: display_path(&group.input),
                label: file_label(&group.input),
                status: JobProgressItemStatus::Queued,
                progress: 0,
                message: "等待处理。".to_string(),
                current_target: None,
                frame: None,
                total_frames: None,
                completed_targets: 0,
                total_targets: group.items.len(),
                artifacts: Vec::new(),
                error: None,
            })
            .collect();
        snapshot.message = format!("视频处理开始，{} 个视频等待处理。", groups.len());
    });

    let mut artifacts = Vec::new();
    let mut failures = Vec::new();
    let mut remaining = groups.into_iter();

    loop {
        if job_is_cancelled(&state, &job_id) {
            return;
        }

        let mut handles = Vec::new();
        for _ in 0..concurrency {
            let Some(group) = remaining.next() else {
                break;
            };
            let app = app.clone();
            let state = state.clone();
            let job_id = job_id.clone();
            let ffmpeg_path = ffmpeg_path.clone();
            let ffprobe_path = ffprobe_path.clone();
            handles.push(std::thread::spawn(move || {
                execute_video_group(
                    app,
                    state,
                    job_id,
                    ffmpeg_path,
                    av1_encoders,
                    ffprobe_path,
                    group,
                )
            }));
        }

        if handles.is_empty() {
            break;
        }

        for handle in handles {
            let Ok(result) = handle.join() else {
                failures.push(MediaFailure {
                    output: "视频处理线程".to_string(),
                    detail: "视频处理线程异常退出".to_string(),
                });
                continue;
            };
            artifacts.extend(result.artifacts);
            failures.extend(result.failures);
        }
    }

    finish_media_job(app, state, job_id, artifacts, failures);
}

pub(super) fn video_concurrency_for_groups(groups: &[VideoWorkGroup]) -> usize {
    let requested = groups
        .iter()
        .flat_map(|group| group.items.iter())
        .filter_map(|indexed| match indexed.item.kind {
            MediaWorkKind::VideoAv1 { concurrency, .. } => Some(concurrency),
            _ => None,
        })
        .max()
        .unwrap_or(1)
        .clamp(1, 4);
    requested.min(groups.len().max(1))
}
