/*
 * 核心职责：执行单个视频输入的多个输出目标。
 * 业务痛点：单视频多目标转码包含进度切片、失败聚合和临时文件安装，必须从任务调度中隔离。
 * 能力边界：只处理一个 VideoWorkGroup，不决定任务并发和最终收尾。
 */

use super::*;
pub(super) fn execute_video_group(
    app: AppHandle,
    state: AppState,
    job_id: String,
    ffmpeg_path: PathBuf,
    av1_encoders: Av1EncoderSet,
    ffprobe_path: Option<PathBuf>,
    group: VideoWorkGroup,
) -> VideoGroupResult {
    let mut artifacts = Vec::new();
    let mut failures = Vec::new();
    if job_is_cancelled(&state, &job_id) {
        return VideoGroupResult {
            artifacts,
            failures,
        };
    }

    let _ = update_video_progress_item(&app, &state, &job_id, &group.id, |item| {
        item.status = JobProgressItemStatus::Running;
        item.progress = 0;
        item.message = "正在读取视频帧信息。".to_string();
    });
    let total_frames = ffprobe_path
        .as_deref()
        .and_then(|path| probe_video_frame_count(path, &group.input));
    let mut group_artifacts = 0usize;
    let mut group_failures = 0usize;
    let target_count = group.items.len().max(1);

    let _ = update_video_progress_item(&app, &state, &job_id, &group.id, |item| {
        item.status = JobProgressItemStatus::Running;
        item.progress = 0;
        item.message = "开始处理视频。".to_string();
        item.total_frames = total_frames;
    });

    for (target_index, indexed_item) in group.items.iter().enumerate() {
        if job_is_cancelled(&state, &job_id) {
            return VideoGroupResult {
                artifacts,
                failures,
            };
        }

        let item = &indexed_item.item;
        let group_id = group.id.clone();
        let input_label = file_label(&item.input);
        let target_label = item.label.to_string();
        let slice_start_progress = video_item_progress(target_index, target_count, 0.0);
        let _ = update_video_progress_item(&app, &state, &job_id, &group_id, |progress_item| {
            progress_item.status = JobProgressItemStatus::Running;
            progress_item.current_target = Some(target_label.clone());
            progress_item.progress = slice_start_progress;
            progress_item.frame = None;
            progress_item.message = format!("正在处理：{} -> {}", input_label, target_label);
        });

        let temp_output = temp_output_path(&item.output, &job_id, indexed_item.index);
        cleanup_temp(&temp_output);
        if let Some(parent) = temp_output.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                let detail = error.to_string();
                group_failures += 1;
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail: detail.clone(),
                });
                mark_video_target_failed(
                    &app,
                    &state,
                    &job_id,
                    &group_id,
                    target_index,
                    target_count,
                    &target_label,
                    detail,
                );
                continue;
            }
        }

        let args = match ffmpeg_args_for_item(item, &temp_output, av1_encoders) {
            Ok(args) => args,
            Err(error) => {
                let detail = error.message;
                group_failures += 1;
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail: detail.clone(),
                });
                mark_video_target_failed(
                    &app,
                    &state,
                    &job_id,
                    &group_id,
                    target_index,
                    target_count,
                    &target_label,
                    detail,
                );
                continue;
            }
        };

        let mut last_progress = u8::MAX;
        let mut last_frame = None;
        let mut last_update = Instant::now();
        let outcome = run_ffmpeg_command(
            &state,
            &job_id,
            &ffmpeg_path,
            &args,
            &temp_output,
            |progress| {
                let Some(frame) = progress.frame else {
                    return;
                };
                let target_fraction = total_frames
                    .map(|total| (frame as f64 / total.max(1) as f64).clamp(0.0, 0.995))
                    .unwrap_or(0.0);
                let next_progress =
                    video_item_progress(target_index, target_count, target_fraction);
                if next_progress == last_progress
                    && Some(frame) == last_frame
                    && last_update.elapsed() < Duration::from_millis(500)
                {
                    return;
                }
                last_progress = next_progress;
                last_frame = Some(frame);
                last_update = Instant::now();
                let _ =
                    update_video_progress_item(&app, &state, &job_id, &group_id, |progress_item| {
                        progress_item.status = JobProgressItemStatus::Running;
                        progress_item.current_target = Some(target_label.clone());
                        progress_item.progress = next_progress;
                        progress_item.frame = Some(frame);
                        progress_item.total_frames = total_frames;
                        progress_item.message =
                            format!("正在处理：{} -> {}", input_label, target_label);
                    });
            },
        );

        match outcome {
            FfmpegRunOutcome::Succeeded => {
                if job_is_cancelled(&state, &job_id) {
                    cleanup_temp(&temp_output);
                    return VideoGroupResult {
                        artifacts,
                        failures,
                    };
                }
                match install_output(&temp_output, &item.output) {
                    Ok(()) => {
                        let artifact = display_path(&item.output);
                        artifacts.push(artifact.clone());
                        group_artifacts += 1;
                        mark_video_target_completed(
                            &app,
                            &state,
                            &job_id,
                            &group_id,
                            target_index,
                            target_count,
                            &target_label,
                            Some(artifact),
                        );
                    }
                    Err(error) => {
                        cleanup_temp(&temp_output);
                        let detail = error.to_string();
                        group_failures += 1;
                        failures.push(MediaFailure {
                            output: display_path(&item.output),
                            detail: detail.clone(),
                        });
                        mark_video_target_failed(
                            &app,
                            &state,
                            &job_id,
                            &group_id,
                            target_index,
                            target_count,
                            &target_label,
                            detail,
                        );
                    }
                }
            }
            FfmpegRunOutcome::Failed(detail) => {
                cleanup_temp(&temp_output);
                group_failures += 1;
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail: detail.clone(),
                });
                mark_video_target_failed(
                    &app,
                    &state,
                    &job_id,
                    &group_id,
                    target_index,
                    target_count,
                    &target_label,
                    detail,
                );
            }
            FfmpegRunOutcome::Cancelled => {
                cleanup_temp(&temp_output);
                return VideoGroupResult {
                    artifacts,
                    failures,
                };
            }
        }
    }

    let _ = update_video_progress_item(&app, &state, &job_id, &group.id, |item| {
        item.current_target = None;
        item.frame = None;
        item.progress = 100;
        if group_artifacts > 0 {
            item.status = JobProgressItemStatus::Succeeded;
            item.message = if group_failures == 0 {
                format!("完成，生成 {} 个产物。", group_artifacts)
            } else {
                format!(
                    "完成，生成 {} 个产物，{} 个目标失败。",
                    group_artifacts, group_failures
                )
            };
        } else {
            item.status = JobProgressItemStatus::Failed;
            item.message = "该视频未生成任何产物。".to_string();
        }
    });

    VideoGroupResult {
        artifacts,
        failures,
    }
}

pub(super) struct VideoGroupResult {
    pub(super) artifacts: Vec<String>,
    pub(super) failures: Vec<MediaFailure>,
}
