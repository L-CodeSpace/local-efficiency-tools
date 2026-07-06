/*
 * 核心职责：探测视频帧数和帧率。
 * 业务痛点：进度估算依赖 ffprobe，失败时必须降级而不能阻断转码。
 * 能力边界：只提供视频进度估算所需的探测解析。
 */

use super::*;

pub(super) fn probe_video_frame_count(ffprobe_path: &Path, input: &Path) -> Option<u64> {
    probe_video_stream(ffprobe_path, input, false)
        .and_then(frame_count_from_probe)
        .or_else(|| probe_video_stream(ffprobe_path, input, true).and_then(frame_count_from_probe))
}

pub(super) fn probe_video_stream(
    ffprobe_path: &Path,
    input: &Path,
    count_frames: bool,
) -> Option<FfprobeOutput> {
    let mut command = hidden_command(ffprobe_path);
    command
        .arg("-v")
        .arg("error")
        .arg("-select_streams")
        .arg("v:0");
    if count_frames {
        command.arg("-count_frames");
    }
    command
        .arg("-show_entries")
        .arg("stream=nb_read_frames,nb_frames,avg_frame_rate,r_frame_rate,duration")
        .arg("-of")
        .arg("json")
        .arg(input);
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice::<FfprobeOutput>(&output.stdout).ok()
}

pub(super) fn frame_count_from_probe(output: FfprobeOutput) -> Option<u64> {
    let stream = output.streams.into_iter().next()?;
    stream
        .nb_read_frames
        .as_deref()
        .and_then(parse_positive_u64)
        .or_else(|| stream.nb_frames.as_deref().and_then(parse_positive_u64))
        .or_else(|| {
            let duration = stream.duration.as_deref()?.parse::<f64>().ok()?;
            let fps = stream
                .avg_frame_rate
                .as_deref()
                .and_then(parse_frame_rate)
                .or_else(|| stream.r_frame_rate.as_deref().and_then(parse_frame_rate))?;
            let frames = (duration * fps).round();
            (frames.is_finite() && frames > 0.0).then_some(frames as u64)
        })
}

pub(super) fn parse_positive_u64(value: &str) -> Option<u64> {
    let parsed = value.trim().parse::<u64>().ok()?;
    (parsed > 0).then_some(parsed)
}

pub(super) fn parse_frame_rate(value: &str) -> Option<f64> {
    let value = value.trim();
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.parse::<f64>().ok()?;
        let denominator = denominator.parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        let rate = numerator / denominator;
        return (rate.is_finite() && rate > 0.0).then_some(rate);
    }
    let rate = value.parse::<f64>().ok()?;
    (rate.is_finite() && rate > 0.0).then_some(rate)
}
