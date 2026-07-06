/*
 * 核心职责：定位 FFmpeg 与 ffprobe 运行时。
 * 业务痛点：运行时发现和下载状态不能散落在任务执行逻辑里。
 * 能力边界：只负责可执行文件解析和运行时状态。
 */

use super::*;

pub(super) fn ffmpeg_path_for_execution(app: &AppHandle) -> AppResult<PathBuf> {
    let local_path = ffmpeg_binary_path(app)?;
    if let Some(path) = executable_file(&local_path) {
        return Ok(path);
    }
    resolve_executable_path("ffmpeg").ok_or_else(|| {
        AppError::new(
            "media_runtime_missing",
            "未检测到 FFmpeg，无法执行图片或视频处理",
        )
    })
}

pub(super) fn ffprobe_path_for_execution(app: &AppHandle) -> AppResult<PathBuf> {
    let local_path = ffprobe_binary_path(app)?;
    if let Some(path) = executable_file(&local_path) {
        return Ok(path);
    }
    resolve_executable_path("ffprobe").ok_or_else(|| {
        AppError::new(
            "media_probe_runtime_missing",
            "未检测到 ffprobe，无法查看视频详情",
        )
    })
}

pub(super) fn detect_av1_encoders(ffmpeg_path: &Path) -> Av1EncoderSet {
    let output = hidden_command(ffmpeg_path)
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .ok();
    let Some(output) = output else {
        return Av1EncoderSet::default();
    };
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    Av1EncoderSet {
        nvenc: text.contains("av1_nvenc"),
        svt: text.contains("libsvtav1"),
        aom: text.contains("libaom-av1"),
    }
}

pub(super) fn resolve_av1_encoder(
    requested: VideoAv1Encoder,
    available: Av1EncoderSet,
) -> Option<Av1Encoder> {
    match requested {
        VideoAv1Encoder::Av1Nvenc if available.nvenc => Some(Av1Encoder::Nvenc),
        VideoAv1Encoder::LibSvtAv1 if available.svt => Some(Av1Encoder::SvtAv1),
        VideoAv1Encoder::LibAomAv1 if available.aom => Some(Av1Encoder::AomAv1),
        VideoAv1Encoder::Auto => {
            if available.nvenc {
                Some(Av1Encoder::Nvenc)
            } else if available.svt {
                Some(Av1Encoder::SvtAv1)
            } else if available.aom {
                Some(Av1Encoder::AomAv1)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn filter_chain(hqdn3d: Option<Hqdn3d>, corner_radius: Option<u32>) -> Option<String> {
    let mut filters = Vec::new();
    if let Some(hqdn3d) = hqdn3d {
        filters.push(format!(
            "hqdn3d={:.1}:{:.1}:{:.1}:{:.1}",
            hqdn3d.luma_spatial,
            hqdn3d.chroma_spatial,
            hqdn3d.luma_temporal,
            hqdn3d.chroma_temporal
        ));
    }
    if let Some(radius) = corner_radius.filter(|value| *value > 0) {
        filters.push(rounded_corner_filter(radius));
    }
    if filters.is_empty() {
        None
    } else {
        Some(filters.join(","))
    }
}

pub(super) fn rounded_corner_filter(radius: u32) -> String {
    format!(
        "format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(gt(abs(W/2-X)\\,W/2-{r})*gt(abs(H/2-Y)\\,H/2-{r})\\,if(lte(hypot({r}-(W/2-abs(W/2-X))\\,{r}-(H/2-abs(H/2-Y)))\\,{r})\\,255\\,0)\\,255)'",
        r = radius
    )
}

pub(super) fn push_video_filter(args: &mut Vec<String>, filter: Option<String>) {
    if let Some(filter) = filter {
        args.push("-vf".to_string());
        args.push(filter);
    }
}

pub(super) fn image_input_format_override(item: &MediaWorkItem) -> Option<&'static str> {
    if !matches!(
        item.kind,
        MediaWorkKind::ImageWebp { .. } | MediaWorkKind::ImageAvif { .. }
    ) {
        return None;
    }
    detect_jpeg_magic(&item.input).then_some("mjpeg")
}

pub(super) fn detect_jpeg_magic(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 3];
    file.read_exact(&mut header).is_ok() && header == [0xFF, 0xD8, 0xFF]
}

pub(super) fn hqdn3d_from_request(request: &MediaPlanRequest) -> Option<Hqdn3d> {
    let hqdn3d = Hqdn3d {
        luma_spatial: request.hqdn_luma_spatial.unwrap_or(0.0).max(0.0),
        chroma_spatial: request.hqdn_chroma_spatial.unwrap_or(0.0).max(0.0),
        luma_temporal: request.hqdn_luma_temporal.unwrap_or(0.0).max(0.0),
        chroma_temporal: request.hqdn_chroma_temporal.unwrap_or(0.0).max(0.0),
    };
    if hqdn3d.luma_spatial > 0.0
        || hqdn3d.chroma_spatial > 0.0
        || hqdn3d.luma_temporal > 0.0
        || hqdn3d.chroma_temporal > 0.0
    {
        Some(hqdn3d)
    } else {
        None
    }
}

pub(super) fn avif_crf(quality: u8) -> u8 {
    63u8.saturating_sub(((quality as f32 / 100.0) * 63.0).round() as u8)
}

pub(super) fn clamp_u8(value: Option<u8>, default: u8, min: u8, max: u8) -> u8 {
    value.unwrap_or(default).clamp(min, max)
}

pub(super) fn clamp_usize(value: Option<usize>, default: usize, min: usize, max: usize) -> usize {
    value.unwrap_or(default).clamp(min, max)
}

pub(super) fn parse_positive_u32(value: Option<&str>) -> Option<u32> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
}

pub(super) fn progress_for(done: usize, total: usize) -> u8 {
    ((done.saturating_mul(100) / total.max(1)).min(99)) as u8
}

pub(super) fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| display_path(path))
}

pub(super) fn job_is_cancelled(state: &AppState, job_id: &str) -> bool {
    matches!(
        get_job(state, job_id),
        Ok(Some(snapshot)) if snapshot.status == JobStatus::Cancelled
    )
}

pub(super) fn kill_registered_process(state: &AppState, job_id: &str) {
    if let Ok(mut processes) = state.job_processes.lock() {
        if let Some(children) = processes.remove(job_id) {
            for mut child in children {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

pub(super) fn failure_summary(failures: &[MediaFailure]) -> String {
    if failures.is_empty() {
        return String::new();
    }
    failures
        .iter()
        .take(5)
        .map(|failure| format!("{}: {}", failure.output, failure.detail))
        .collect::<Vec<_>>()
        .join("\n")
}
