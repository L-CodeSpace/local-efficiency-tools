/*
 * 核心职责：解析进度并安装输出文件。
 * 业务痛点：临时产物安装与失败收尾必须集中处理，避免残留文件。
 * 能力边界：只处理进度解析、产物路径和收尾清理。
 */

use super::*;

pub(super) fn parse_ffmpeg_progress_line(line: &str) -> Option<FfmpegProgress> {
    let (key, value) = line.split_once('=')?;
    match key.trim() {
        "frame" => value
            .trim()
            .parse::<u64>()
            .ok()
            .map(|frame| FfmpegProgress { frame: Some(frame) }),
        _ => None,
    }
}

pub(super) fn finish_media_job(
    app: AppHandle,
    state: AppState,
    job_id: String,
    artifacts: Vec<String>,
    failures: Vec<MediaFailure>,
) {
    if job_is_cancelled(&state, &job_id) {
        return;
    }

    if artifacts.is_empty() {
        let detail = failure_summary(&failures);
        let _ = update_job(&app, &state, &job_id, |snapshot| {
            snapshot.status = JobStatus::Failed;
            snapshot.progress = 100;
            snapshot.message = "媒体处理失败，未生成任何产物。".to_string();
            snapshot.error = Some(
                AppError::new("media_job_failed", "媒体处理失败，未生成任何产物")
                    .with_detail(detail),
            );
            snapshot.result = Some(JobResult {
                executor: "ffmpeg".to_string(),
                completed: true,
                artifacts: Vec::new(),
            });
        });
        return;
    }

    let message = if failures.is_empty() {
        format!("媒体处理完成，生成 {} 个产物。", artifacts.len())
    } else {
        format!(
            "媒体处理完成，生成 {} 个产物，{} 个产物失败。",
            artifacts.len(),
            failures.len()
        )
    };
    if !failures.is_empty() {
        observability::emit_error(&app, failure_summary(&failures));
    }
    let _ = update_job(&app, &state, &job_id, |snapshot| {
        snapshot.status = JobStatus::Succeeded;
        snapshot.progress = 100;
        snapshot.message = message;
        snapshot.result = Some(JobResult {
            executor: "ffmpeg".to_string(),
            completed: true,
            artifacts,
        });
    });
}

pub(super) fn output_path_for(
    input: &Path,
    output_root: Option<&Path>,
    source_root: Option<&Path>,
    suffix: &str,
) -> AppResult<PathBuf> {
    if let Some(output_root) = output_root {
        if let Some(source_root) = source_root {
            if let Ok(relative) = input.strip_prefix(source_root) {
                return Ok(output_root.join(path_with_suffix(relative, suffix)?));
            }
        }
        return Ok(output_root.join(file_name_with_suffix(input, suffix)?));
    }

    let parent = input
        .parent()
        .ok_or_else(|| AppError::new("invalid_path", "无法读取媒体输入父目录"))?;
    Ok(parent.join(file_name_with_suffix(input, suffix)?))
}

pub(super) fn path_with_suffix(path: &Path, suffix: &str) -> AppResult<PathBuf> {
    let name = file_name_with_suffix(path, suffix)?;
    Ok(path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(&name))
        .unwrap_or(name))
}

pub(super) fn file_name_with_suffix(path: &Path, suffix: &str) -> AppResult<PathBuf> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::new("invalid_media_name", "媒体文件名无效"))?;
    Ok(PathBuf::from(format!("{stem}{suffix}")))
}

pub(super) fn temp_output_path(output: &Path, job_id: &str, index: usize) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| Path::new(""));
    let stem = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("media");
    let extension = output
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let short_id = &job_id[..8.min(job_id.len())];
    parent.join(format!("{stem}.codex-{short_id}-{index}.tmp{extension}"))
}

pub(super) fn install_output(temp_output: &Path, output: &Path) -> AppResult<()> {
    if !temp_output.is_file() {
        return Err(AppError::new(
            "media_output_missing",
            "FFmpeg 未生成预期输出文件",
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    if output.exists() {
        fs::remove_file(output)?;
    }
    fs::rename(temp_output, output)?;
    Ok(())
}

pub(super) fn cleanup_temp(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}
