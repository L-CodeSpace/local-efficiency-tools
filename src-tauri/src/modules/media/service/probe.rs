/*
 * 核心职责：解析 ffprobe 视频详情。
 * 业务痛点：详情探测与转码执行混在一起会让错误边界难以判断。
 * 能力边界：只负责 ffprobe 输出解析和摘要格式化。
 */

use super::*;

pub fn probe_video(app: &AppHandle, state: &AppState, path: String) -> AppResult<MediaProbeInfo> {
    let input = ensure_allowed_path(app, state, &PathBuf::from(&path))?;
    if !input.is_file() {
        return Err(
            AppError::new("media_probe_input_not_file", "视频详情探测路径必须是文件")
                .with_detail(display_path(&input)),
        );
    }

    let ffprobe_path = ffprobe_path_for_execution(app)?;
    let output = hidden_command(&ffprobe_path)
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(&input)
        .output()
        .map_err(|error| {
            AppError::new("media_probe_start_failed", "启动 ffprobe 失败")
                .with_detail(error.to_string())
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(
            AppError::new("media_probe_failed", "ffprobe 探测视频失败").with_detail(
                if stderr.is_empty() {
                    format!("ffprobe 退出码：{}", output.status)
                } else {
                    stderr
                },
            ),
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_output(&input, &stdout)
}

pub(crate) fn parse_ffprobe_output(path: &Path, output: &str) -> AppResult<MediaProbeInfo> {
    let value = serde_json::from_str::<serde_json::Value>(output).map_err(|error| {
        AppError::new("media_probe_parse_failed", "解析 ffprobe 输出失败")
            .with_detail(error.to_string())
    })?;
    let format = value.get("format");
    let format_name = format.and_then(|value| string_field(value, "format_name"));
    let format_long_name = format.and_then(|value| string_field(value, "format_long_name"));
    let duration_seconds = format.and_then(|value| f64_field(value, "duration"));
    let size_bytes = format.and_then(|value| u64_field(value, "size"));
    let bitrate_bps = format.and_then(|value| u64_field(value, "bit_rate"));
    let streams = value
        .get("streams")
        .and_then(|value| value.as_array())
        .map(|streams| streams.iter().map(parse_ffprobe_stream).collect::<Vec<_>>())
        .unwrap_or_default();
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| display_path(path));
    let raw_summary = probe_summary(
        format_long_name.as_deref().or(format_name.as_deref()),
        duration_seconds,
        size_bytes,
        bitrate_bps,
        streams.len(),
    );

    Ok(MediaProbeInfo {
        path: display_path(path),
        name,
        format_name,
        format_long_name,
        duration_seconds,
        size_bytes,
        bitrate_bps,
        streams,
        raw_summary,
    })
}

pub(super) fn parse_ffprobe_stream(value: &serde_json::Value) -> MediaProbeStream {
    let tags = value.get("tags");
    MediaProbeStream {
        index: u32_field(value, "index").unwrap_or(0),
        codec_type: string_field(value, "codec_type"),
        codec_name: string_field(value, "codec_name"),
        codec_long_name: string_field(value, "codec_long_name"),
        profile: string_field(value, "profile"),
        width: u32_field(value, "width"),
        height: u32_field(value, "height"),
        frame_rate: frame_rate_field(value),
        pixel_format: string_field(value, "pix_fmt"),
        sample_rate: u32_field(value, "sample_rate"),
        channels: u32_field(value, "channels"),
        channel_layout: string_field(value, "channel_layout"),
        duration_seconds: f64_field(value, "duration"),
        bitrate_bps: u64_field(value, "bit_rate"),
        language: tags.and_then(|value| string_field(value, "language")),
        title: tags.and_then(|value| string_field(value, "title")),
    }
}

pub(super) fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::String(text) => normalize_probe_text(text),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

pub(super) fn u32_field(value: &serde_json::Value, key: &str) -> Option<u32> {
    u64_field(value, key).and_then(|value| u32::try_from(value).ok())
}

pub(super) fn u64_field(value: &serde_json::Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_u64(),
        serde_json::Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    })
}

pub(super) fn f64_field(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(|value| match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(text) => text.trim().parse::<f64>().ok(),
        _ => None,
    })
}

pub(super) fn frame_rate_field(value: &serde_json::Value) -> Option<String> {
    string_field(value, "avg_frame_rate")
        .or_else(|| string_field(value, "r_frame_rate"))
        .and_then(|value| normalize_frame_rate(&value))
}

pub(super) fn normalize_probe_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("N/A") {
        None
    } else {
        Some(text.to_string())
    }
}

pub(super) fn normalize_frame_rate(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value == "0/0" || value.eq_ignore_ascii_case("N/A") {
        return None;
    }
    if let Some((left, right)) = value.split_once('/') {
        let numerator = left.trim().parse::<f64>().ok()?;
        let denominator = right.trim().parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        return Some(format!("{:.3} fps", numerator / denominator));
    }
    Some(value.to_string())
}

pub(super) fn probe_summary(
    format: Option<&str>,
    duration_seconds: Option<f64>,
    size_bytes: Option<u64>,
    bitrate_bps: Option<u64>,
    stream_count: usize,
) -> String {
    [
        format.map(ToOwned::to_owned),
        duration_seconds.map(format_duration_label),
        size_bytes.map(format_size_label),
        bitrate_bps.map(format_bitrate_label),
        Some(format!("{stream_count} 个流")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

pub(super) fn format_duration_label(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

pub(super) fn format_size_label(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub(super) fn format_bitrate_label(bits_per_second: u64) -> String {
    if bits_per_second >= 1_000_000 {
        format!("{:.2} Mbps", bits_per_second as f64 / 1_000_000.0)
    } else if bits_per_second >= 1_000 {
        format!("{:.0} Kbps", bits_per_second as f64 / 1_000.0)
    } else {
        format!("{bits_per_second} bps")
    }
}
