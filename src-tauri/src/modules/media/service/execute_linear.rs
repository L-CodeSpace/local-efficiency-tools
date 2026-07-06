/*
 * 核心职责：执行线性媒体任务。
 * 业务痛点：图片和简单任务的执行流程需要与视频多目标进度隔离。
 * 能力边界：只处理非视频分组的顺序执行。
 */

use super::*;

pub(super) fn execute_media_job(
    app: AppHandle,
    state: AppState,
    job_id: String,
    ffmpeg_path: PathBuf,
    kind: MediaJobKind,
    work: Vec<MediaWorkItem>,
) {
    match kind {
        MediaJobKind::ImageCompression => {
            execute_linear_media_job(app, state, job_id, ffmpeg_path, work)
        }
        MediaJobKind::VideoTranscode => {
            execute_video_media_job(app, state, job_id, ffmpeg_path, work)
        }
    }
}

pub(super) fn execute_linear_media_job(
    app: AppHandle,
    state: AppState,
    job_id: String,
    ffmpeg_path: PathBuf,
    work: Vec<MediaWorkItem>,
) {
    let total = work.len().max(1);
    let av1_encoders = detect_av1_encoders(&ffmpeg_path);
    let mut artifacts = Vec::new();
    let mut failures = Vec::new();

    for (index, item) in work.iter().enumerate() {
        if job_is_cancelled(&state, &job_id) {
            return;
        }

        let _ = update_job(&app, &state, &job_id, |snapshot| {
            snapshot.progress = progress_for(index, total);
            snapshot.message = format!(
                "正在处理 {}/{}：{} -> {}",
                index + 1,
                total,
                file_label(&item.input),
                item.label
            );
        });

        let temp_output = temp_output_path(&item.output, &job_id, index);
        cleanup_temp(&temp_output);
        if let Some(parent) = temp_output.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail: error.to_string(),
                });
                continue;
            }
        }

        let args = match ffmpeg_args_for_item(item, &temp_output, av1_encoders) {
            Ok(args) => args,
            Err(error) => {
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail: error.message,
                });
                continue;
            }
        };

        match run_ffmpeg_command(&state, &job_id, &ffmpeg_path, &args, &temp_output, |_| {}) {
            FfmpegRunOutcome::Succeeded => {
                if job_is_cancelled(&state, &job_id) {
                    cleanup_temp(&temp_output);
                    return;
                }
                match install_output(&temp_output, &item.output) {
                    Ok(()) => artifacts.push(display_path(&item.output)),
                    Err(error) => {
                        cleanup_temp(&temp_output);
                        failures.push(MediaFailure {
                            output: display_path(&item.output),
                            detail: error.to_string(),
                        });
                    }
                }
            }
            FfmpegRunOutcome::Failed(detail) => {
                cleanup_temp(&temp_output);
                failures.push(MediaFailure {
                    output: display_path(&item.output),
                    detail,
                });
            }
            FfmpegRunOutcome::Cancelled => {
                cleanup_temp(&temp_output);
                return;
            }
        }

        let _ = update_job(&app, &state, &job_id, |snapshot| {
            snapshot.progress = progress_for(index + 1, total);
        });
    }

    finish_media_job(app, state, job_id, artifacts, failures);
}
