/*
 * 核心职责：验证媒体探测解析。
 * 业务痛点：拆分后仍需保留关键解析行为的回归测试。
 * 能力边界：只包含媒体模块内部单元测试。
 */

#[cfg(test)]
mod tests {
    use crate::modules::media::service::parse_ffprobe_output;
    use std::path::Path;

    #[test]
    fn parses_video_and_audio_probe_output() {
        let json = r#"{
          "streams": [
            {
              "index": 0,
              "codec_name": "h264",
              "codec_long_name": "H.264 / AVC",
              "profile": "High",
              "codec_type": "video",
              "width": 1920,
              "height": 1080,
              "avg_frame_rate": "30000/1001",
              "pix_fmt": "yuv420p",
              "duration": "10.500000",
              "bit_rate": "5000000",
              "tags": { "language": "und", "title": "Main" }
            },
            {
              "index": 1,
              "codec_name": "aac",
              "codec_long_name": "AAC",
              "codec_type": "audio",
              "sample_rate": "48000",
              "channels": 2,
              "channel_layout": "stereo",
              "duration": "10.500000",
              "bit_rate": "128000",
              "tags": { "language": "eng" }
            }
          ],
          "format": {
            "format_name": "mov,mp4,m4a,3gp,3g2,mj2",
            "format_long_name": "QuickTime / MOV",
            "duration": "10.500000",
            "size": "6600000",
            "bit_rate": "5028571"
          }
        }"#;

        let info = parse_ffprobe_output(Path::new("C:/video/sample.mp4"), json).unwrap();

        assert_eq!(info.name, "sample.mp4");
        assert_eq!(info.format_long_name.as_deref(), Some("QuickTime / MOV"));
        assert_eq!(info.duration_seconds, Some(10.5));
        assert_eq!(info.size_bytes, Some(6_600_000));
        assert_eq!(info.bitrate_bps, Some(5_028_571));
        assert_eq!(info.streams.len(), 2);
        assert_eq!(info.streams[0].codec_type.as_deref(), Some("video"));
        assert_eq!(info.streams[0].width, Some(1920));
        assert_eq!(info.streams[0].height, Some(1080));
        assert_eq!(info.streams[0].frame_rate.as_deref(), Some("29.970 fps"));
        assert_eq!(info.streams[1].codec_type.as_deref(), Some("audio"));
        assert_eq!(info.streams[1].sample_rate, Some(48000));
        assert_eq!(info.streams[1].channels, Some(2));
    }

    #[test]
    fn parses_audio_only_probe_output() {
        let json = r#"{
          "streams": [
            {
              "index": 0,
              "codec_name": "mp3",
              "codec_long_name": "MP3",
              "codec_type": "audio",
              "sample_rate": "44100",
              "channels": 2,
              "duration": "90.000000"
            }
          ],
          "format": {
            "format_name": "mp3",
            "format_long_name": "MP2/3 (MPEG audio layer 2/3)",
            "duration": "90.000000",
            "size": "1440000"
          }
        }"#;

        let info = parse_ffprobe_output(Path::new("/tmp/audio.mp3"), json).unwrap();

        assert_eq!(info.streams.len(), 1);
        assert_eq!(info.streams[0].codec_type.as_deref(), Some("audio"));
        assert_eq!(info.streams[0].codec_name.as_deref(), Some("mp3"));
        assert_eq!(info.bitrate_bps, None);
    }

    #[test]
    fn parses_probe_output_with_missing_duration_and_bitrate() {
        let json = r#"{
          "streams": [
            {
              "index": 0,
              "codec_type": "video",
              "codec_name": "vp9",
              "width": 1280,
              "height": 720,
              "avg_frame_rate": "0/0",
              "bit_rate": "N/A"
            }
          ],
          "format": {
            "format_name": "matroska,webm",
            "duration": "N/A",
            "bit_rate": "N/A"
          }
        }"#;

        let info = parse_ffprobe_output(Path::new("/tmp/no-duration.webm"), json).unwrap();

        assert_eq!(info.duration_seconds, None);
        assert_eq!(info.bitrate_bps, None);
        assert_eq!(info.streams[0].frame_rate, None);
        assert_eq!(info.streams[0].bitrate_bps, None);
    }
}
