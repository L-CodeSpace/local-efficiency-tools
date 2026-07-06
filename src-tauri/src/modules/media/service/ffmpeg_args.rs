/*
 * 核心职责：生成并运行 FFmpeg 命令。
 * 业务痛点：命令参数是转码正确性的核心边界，必须脱离任务调度。
 * 能力边界：只构造参数并执行单个 FFmpeg 进程。
 */

use super::*;

pub(super) fn ffmpeg_args_for_item(
    item: &MediaWorkItem,
    output: &Path,
    av1_encoders: Av1EncoderSet,
) -> AppResult<Vec<String>> {
    let mut args = vec![
        "-y".to_string(),
        "-nostdin".to_string(),
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-progress".to_string(),
        "pipe:1".to_string(),
        "-nostats".to_string(),
    ];
    if let Some(format) = image_input_format_override(item) {
        args.push("-f".to_string());
        args.push(format.to_string());
    }
    args.extend(["-i".to_string(), item.input.to_string_lossy().to_string()]);

    match &item.kind {
        MediaWorkKind::ImageWebp {
            quality,
            corner_radius,
        } => {
            push_video_filter(&mut args, filter_chain(None, *corner_radius));
            args.extend([
                "-frames:v".to_string(),
                "1".to_string(),
                "-c:v".to_string(),
                "libwebp".to_string(),
                "-quality".to_string(),
                quality.to_string(),
            ]);
        }
        MediaWorkKind::ImageAvif {
            quality,
            corner_radius,
        } => {
            push_video_filter(&mut args, filter_chain(None, *corner_radius));
            args.extend([
                "-frames:v".to_string(),
                "1".to_string(),
                "-c:v".to_string(),
                "libaom-av1".to_string(),
                "-still-picture".to_string(),
                "1".to_string(),
                "-crf".to_string(),
                avif_crf(*quality).to_string(),
                "-b:v".to_string(),
                "0".to_string(),
            ]);
        }
        MediaWorkKind::VideoAnimatedWebp {
            quality,
            corner_radius,
            hqdn3d,
        } => {
            push_video_filter(&mut args, filter_chain(*hqdn3d, *corner_radius));
            args.extend([
                "-an".to_string(),
                "-c:v".to_string(),
                "libwebp".to_string(),
                "-loop".to_string(),
                "0".to_string(),
                "-quality".to_string(),
                quality.to_string(),
                "-compression_level".to_string(),
                "6".to_string(),
            ]);
        }
        MediaWorkKind::VideoAv1 {
            with_audio,
            encoder,
            speed,
            crf,
            threads,
            tile_columns,
            tile_rows,
            hqdn3d,
            ..
        } => {
            let encoder = resolve_av1_encoder(*encoder, av1_encoders).ok_or_else(|| {
                AppError::new(
                    "media_av1_encoder_missing",
                    "当前 FFmpeg 未提供所选 AV1 编码器",
                )
            })?;
            args.extend(["-map".to_string(), "0:v:0".to_string()]);
            if *with_audio {
                args.extend(["-map".to_string(), "0:a?".to_string()]);
            }
            push_video_filter(&mut args, filter_chain(*hqdn3d, None));
            match encoder {
                Av1Encoder::Nvenc => args.extend([
                    "-c:v".to_string(),
                    "av1_nvenc".to_string(),
                    "-preset".to_string(),
                    nvenc_preset(*speed),
                    "-tune".to_string(),
                    "hq".to_string(),
                    "-rc".to_string(),
                    "vbr".to_string(),
                    "-cq".to_string(),
                    crf.to_string(),
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
                ]),
                Av1Encoder::SvtAv1 => args.extend([
                    "-c:v".to_string(),
                    "libsvtav1".to_string(),
                    "-preset".to_string(),
                    speed.to_string(),
                    "-crf".to_string(),
                    crf.to_string(),
                ]),
                Av1Encoder::AomAv1 => args.extend([
                    "-c:v".to_string(),
                    "libaom-av1".to_string(),
                    "-cpu-used".to_string(),
                    speed.to_string(),
                    "-crf".to_string(),
                    crf.to_string(),
                    "-b:v".to_string(),
                    "0".to_string(),
                    "-threads".to_string(),
                    threads.to_string(),
                    "-row-mt".to_string(),
                    "1".to_string(),
                    "-tile-columns".to_string(),
                    tile_columns.to_string(),
                    "-tile-rows".to_string(),
                    tile_rows.to_string(),
                ]),
            }
            args.extend(["-pix_fmt".to_string(), "yuv420p".to_string()]);
            if *with_audio {
                args.extend([
                    "-c:a".to_string(),
                    "aac".to_string(),
                    "-b:a".to_string(),
                    "160k".to_string(),
                ]);
            } else {
                args.push("-an".to_string());
            }
            args.extend(["-movflags".to_string(), "+faststart".to_string()]);
        }
        MediaWorkKind::AudioMp3 => {
            args.extend([
                "-map".to_string(),
                "0:a:0".to_string(),
                "-vn".to_string(),
                "-c:a".to_string(),
                "libmp3lame".to_string(),
                "-q:a".to_string(),
                "2".to_string(),
            ]);
        }
    }

    args.push(output.to_string_lossy().to_string());
    Ok(args)
}

pub(super) fn nvenc_preset(speed: u8) -> String {
    format!("p{}", speed.clamp(1, 7))
}
