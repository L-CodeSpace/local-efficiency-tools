/*
 * 核心职责：构建媒体处理工作项。
 * 业务痛点：输入路径、输出目录和目标格式组合逻辑必须集中校验。
 * 能力边界：只把请求转换为可执行工作项。
 */

use super::*;

pub(super) fn build_media_work(
    app: &AppHandle,
    state: &AppState,
    request: &MediaPlanRequest,
) -> AppResult<Vec<MediaWorkItem>> {
    let source_root = match request
        .source_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(root) => {
            let root = ensure_allowed_path(app, state, &PathBuf::from(root))?;
            if !root.is_dir() {
                return Err(AppError::new(
                    "media_source_root_not_directory",
                    "媒体来源根路径必须是目录",
                ));
            }
            Some(root)
        }
        None => None,
    };
    let output_root = match request
        .output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(output_dir) => Some(ensure_allowed_path(app, state, &PathBuf::from(output_dir))?),
        None => None,
    };

    let mut work = Vec::new();
    for raw_input in &request.inputs {
        let input = ensure_allowed_path(app, state, &PathBuf::from(raw_input))?;
        if !input.is_file() {
            return Err(
                AppError::new("media_input_not_file", "媒体输入路径必须是文件")
                    .with_detail(display_path(&input)),
            );
        }

        match request.kind {
            MediaJobKind::ImageCompression => {
                let format = request
                    .image_format
                    .clone()
                    .unwrap_or(ImageOutputFormat::Webp);
                let quality = clamp_u8(request.quality, 82, 1, 100);
                let corner_radius = parse_positive_u32(request.corner_radius.as_deref());
                let (suffix, label, kind) = match format {
                    ImageOutputFormat::Webp => (
                        ".webp",
                        "WebP 图片",
                        MediaWorkKind::ImageWebp {
                            quality,
                            corner_radius,
                        },
                    ),
                    ImageOutputFormat::Avif => (
                        ".avif",
                        "AVIF 图片",
                        MediaWorkKind::ImageAvif {
                            quality,
                            corner_radius,
                        },
                    ),
                };
                work.push(MediaWorkItem {
                    output: output_path_for(
                        &input,
                        output_root.as_deref(),
                        source_root.as_deref(),
                        suffix,
                    )?,
                    input,
                    label,
                    kind,
                });
            }
            MediaJobKind::VideoTranscode => {
                let targets = request.video_targets.clone().unwrap_or_default();
                let webp_quality = clamp_u8(request.webp_quality, 82, 1, 100);
                let av1_encoder = request.av1_encoder.unwrap_or(VideoAv1Encoder::Auto);
                let av1_speed = clamp_u8(request.av1_speed, 6, 0, 8);
                let av1_crf = clamp_u8(request.av1_crf, 34, 0, 63);
                let av1_threads = clamp_usize(request.av1_threads, 12, 1, 64);
                let av1_tile_columns = clamp_u8(request.av1_tile_columns, 2, 0, 6);
                let av1_tile_rows = clamp_u8(request.av1_tile_rows, 1, 0, 6);
                let video_concurrency = clamp_usize(request.video_concurrency, 1, 1, 4);
                let corner_radius = parse_positive_u32(request.corner_radius.as_deref());
                let hqdn3d = hqdn3d_from_request(request);

                for target in targets {
                    let (suffix, label, kind) = match target {
                        VideoTarget::AnimatedWebp => (
                            ".webp",
                            "WebP 动画",
                            MediaWorkKind::VideoAnimatedWebp {
                                quality: webp_quality,
                                corner_radius,
                                hqdn3d,
                            },
                        ),
                        VideoTarget::Av1WithAudio => (
                            ".av1.mp4",
                            "AV1 视频",
                            MediaWorkKind::VideoAv1 {
                                with_audio: true,
                                encoder: av1_encoder,
                                speed: av1_speed,
                                crf: av1_crf,
                                threads: av1_threads,
                                tile_columns: av1_tile_columns,
                                tile_rows: av1_tile_rows,
                                concurrency: video_concurrency,
                                hqdn3d,
                            },
                        ),
                        VideoTarget::Av1VideoOnly => (
                            ".av1-no-audio.mp4",
                            "AV1 无音轨视频",
                            MediaWorkKind::VideoAv1 {
                                with_audio: false,
                                encoder: av1_encoder,
                                speed: av1_speed,
                                crf: av1_crf,
                                threads: av1_threads,
                                tile_columns: av1_tile_columns,
                                tile_rows: av1_tile_rows,
                                concurrency: video_concurrency,
                                hqdn3d,
                            },
                        ),
                        VideoTarget::AudioMp3 => (".mp3", "MP3 音频", MediaWorkKind::AudioMp3),
                    };
                    work.push(MediaWorkItem {
                        output: output_path_for(
                            &input,
                            output_root.as_deref(),
                            source_root.as_deref(),
                            suffix,
                        )?,
                        input: input.clone(),
                        label,
                        kind,
                    });
                }
            }
        }
    }

    if work.is_empty() {
        return Err(AppError::new(
            "media_outputs_empty",
            "没有可执行的媒体输出任务",
        ));
    }
    Ok(work)
}
