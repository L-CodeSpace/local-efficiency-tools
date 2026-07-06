/*
 * 核心职责：识别媒体处理设备能力并生成 FFmpeg 推荐参数。
 * 业务痛点：编码器选择、线程数和并发数不能依赖前端硬编码。
 * 能力边界：只探测本机硬件与 FFmpeg 能力，不执行转码任务。
 */

use super::*;

pub fn performance_profile(app: &AppHandle) -> AppResult<MediaPerformanceProfile> {
    let device = media_device_summary();
    let ffmpeg_path = ffmpeg_path_for_execution(app).ok();
    let encoder_text = ffmpeg_path
        .as_deref()
        .and_then(ffmpeg_encoder_listing)
        .unwrap_or_default();
    let encoders = media_encoder_capabilities(&encoder_text);
    let recommended = recommended_media_settings(&device, &encoders);
    let message = if recommended.av1_encoder == VideoAv1Encoder::Av1Nvenc {
        "已识别 NVIDIA AV1 硬件编码，推荐优先使用 GPU 编码。".to_string()
    } else if recommended.av1_encoder == VideoAv1Encoder::LibSvtAv1 {
        "已识别 SVT-AV1 软件编码，推荐使用 CPU 高压缩模式。".to_string()
    } else if recommended.av1_encoder == VideoAv1Encoder::LibAomAv1 {
        "未发现硬件 AV1 编码器，已回退到 libaom-av1 软件编码。".to_string()
    } else {
        "未检测到可用 AV1 编码器，请检查 FFmpeg 运行时。".to_string()
    };

    Ok(MediaPerformanceProfile {
        device,
        encoders,
        recommended,
        message,
    })
}

pub(super) fn media_device_summary() -> MediaDeviceSummary {
    let system = sysinfo::System::new_all();
    let cpu_name = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            system
                .cpus()
                .first()
                .map(|cpu| cpu.name().to_string())
                .filter(|name| !name.trim().is_empty())
        })
        .unwrap_or_else(|| "未知处理器".to_string());
    let cpu_logical_cores = system.cpus().len().max(1);
    let cpu_physical_cores = sysinfo::System::physical_core_count()
        .unwrap_or(cpu_logical_cores)
        .max(1);

    MediaDeviceSummary {
        cpu_name,
        cpu_physical_cores,
        cpu_logical_cores,
        ram_total: system.total_memory(),
        ram_available: system.available_memory(),
        gpus: media_gpu_summaries(),
    }
}

#[cfg(windows)]
pub(super) fn media_gpu_summaries() -> Vec<MediaGpuSummary> {
    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; Get-CimInstance Win32_VideoController | ForEach-Object { \"$($_.Name)|$($_.AdapterRAM)|$($_.DriverVersion)\" }",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('|');
            let name = parts.next()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let vram_bytes = parts
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|value| *value > 0);
            let driver_version = parts
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            Some(MediaGpuSummary {
                name,
                vram_bytes,
                driver_version,
            })
        })
        .collect()
}

#[cfg(not(windows))]
pub(super) fn media_gpu_summaries() -> Vec<MediaGpuSummary> {
    Vec::new()
}

pub(super) fn ffmpeg_encoder_listing(ffmpeg_path: &Path) -> Option<String> {
    let output = hidden_command(ffmpeg_path)
        .arg("-hide_banner")
        .arg("-encoders")
        .output()
        .ok()?;
    Some(format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

pub(super) fn media_encoder_capabilities(text: &str) -> Vec<MediaEncoderCapability> {
    let lower = text.to_ascii_lowercase();
    vec![
        MediaEncoderCapability {
            encoder: VideoAv1Encoder::Av1Nvenc,
            ffmpeg_name: "av1_nvenc".to_string(),
            available: lower.contains("av1_nvenc"),
            hardware: true,
            description: "NVIDIA NVENC AV1，速度优先，适合 RTX 40 系列。".to_string(),
        },
        MediaEncoderCapability {
            encoder: VideoAv1Encoder::LibSvtAv1,
            ffmpeg_name: "libsvtav1".to_string(),
            available: lower.contains("libsvtav1"),
            hardware: false,
            description: "SVT-AV1 软件编码，多线程效率较好。".to_string(),
        },
        MediaEncoderCapability {
            encoder: VideoAv1Encoder::LibAomAv1,
            ffmpeg_name: "libaom-av1".to_string(),
            available: lower.contains("libaom-av1"),
            hardware: false,
            description: "libaom-av1 软件编码，兼容性好但速度较慢。".to_string(),
        },
    ]
}

pub(super) fn recommended_media_settings(
    device: &MediaDeviceSummary,
    encoders: &[MediaEncoderCapability],
) -> MediaRecommendedSettings {
    let has = |encoder| {
        encoders
            .iter()
            .any(|capability| capability.encoder == encoder && capability.available)
    };
    let threads = device.cpu_logical_cores.clamp(1, 32);
    if has(VideoAv1Encoder::Av1Nvenc) {
        return MediaRecommendedSettings {
            av1_encoder: VideoAv1Encoder::Av1Nvenc,
            av1_speed: 5,
            av1_crf: 32,
            av1_threads: threads,
            av1_tile_columns: 2,
            av1_tile_rows: 1,
            video_concurrency: 2,
            summary: "RTX/NVENC 可用：推荐 AV1 硬件编码 p5，优先提升处理速度。".to_string(),
            ffmpeg_args: vec![
                "-c:v".to_string(),
                "av1_nvenc".to_string(),
                "-preset".to_string(),
                "p5".to_string(),
                "-tune".to_string(),
                "hq".to_string(),
                "-rc".to_string(),
                "vbr".to_string(),
                "-cq".to_string(),
                "32".to_string(),
                "-b:v".to_string(),
                "0".to_string(),
                "-multipass".to_string(),
                "qres".to_string(),
                "-rc-lookahead".to_string(),
                "32".to_string(),
                "-spatial-aq".to_string(),
                "1".to_string(),
                "-temporal-aq".to_string(),
                "1".to_string(),
            ],
        };
    }
    if has(VideoAv1Encoder::LibSvtAv1) {
        return MediaRecommendedSettings {
            av1_encoder: VideoAv1Encoder::LibSvtAv1,
            av1_speed: 6,
            av1_crf: 34,
            av1_threads: threads,
            av1_tile_columns: 2,
            av1_tile_rows: 1,
            video_concurrency: 1,
            summary: "SVT-AV1 可用：推荐单路 CPU 编码，避免过度抢占线程。".to_string(),
            ffmpeg_args: vec![
                "-c:v".to_string(),
                "libsvtav1".to_string(),
                "-preset".to_string(),
                "6".to_string(),
                "-crf".to_string(),
                "34".to_string(),
            ],
        };
    }
    if has(VideoAv1Encoder::LibAomAv1) {
        return MediaRecommendedSettings {
            av1_encoder: VideoAv1Encoder::LibAomAv1,
            av1_speed: 6,
            av1_crf: 34,
            av1_threads: threads,
            av1_tile_columns: 2,
            av1_tile_rows: 1,
            video_concurrency: 1,
            summary: "仅 libaom-av1 可用：推荐启用 row-mt 与 tiles 提升 CPU 利用率。".to_string(),
            ffmpeg_args: vec![
                "-c:v".to_string(),
                "libaom-av1".to_string(),
                "-cpu-used".to_string(),
                "6".to_string(),
                "-crf".to_string(),
                "34".to_string(),
                "-b:v".to_string(),
                "0".to_string(),
                "-threads".to_string(),
                threads.to_string(),
                "-row-mt".to_string(),
                "1".to_string(),
                "-tile-columns".to_string(),
                "2".to_string(),
                "-tile-rows".to_string(),
                "1".to_string(),
            ],
        };
    }
    MediaRecommendedSettings {
        av1_encoder: VideoAv1Encoder::Auto,
        av1_speed: 6,
        av1_crf: 34,
        av1_threads: threads,
        av1_tile_columns: 2,
        av1_tile_rows: 1,
        video_concurrency: 1,
        summary: "未检测到可用 AV1 编码器。".to_string(),
        ffmpeg_args: Vec::new(),
    }
}
