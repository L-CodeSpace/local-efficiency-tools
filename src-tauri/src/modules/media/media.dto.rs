/*
 * 核心职责：定义媒体处理模块 DTO。
 * 业务痛点：媒体计划、运行时状态、探测结果和转码目标需要稳定 IPC 契约。
 * 能力边界：只描述媒体处理契约，不执行 ffmpeg 或 ffprobe。
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaJobKind {
    ImageCompression,
    VideoTranscode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageOutputFormat {
    Webp,
    Avif,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoTarget {
    AnimatedWebp,
    Av1WithAudio,
    Av1VideoOnly,
    AudioMp3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoAv1Encoder {
    Auto,
    Av1Nvenc,
    LibSvtAv1,
    LibAomAv1,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlanRequest {
    pub kind: MediaJobKind,
    pub inputs: Vec<String>,
    pub output_dir: Option<String>,
    pub source_root: Option<String>,
    pub image_format: Option<ImageOutputFormat>,
    pub quality: Option<u8>,
    pub corner_radius: Option<String>,
    pub max_depth: Option<usize>,
    pub video_targets: Option<Vec<VideoTarget>>,
    pub webp_quality: Option<u8>,
    pub av1_encoder: Option<VideoAv1Encoder>,
    pub av1_speed: Option<u8>,
    pub av1_crf: Option<u8>,
    pub av1_threads: Option<usize>,
    pub av1_tile_columns: Option<u8>,
    pub av1_tile_rows: Option<u8>,
    pub video_concurrency: Option<usize>,
    pub hqdn_luma_spatial: Option<f32>,
    pub hqdn_chroma_spatial: Option<f32>,
    pub hqdn_luma_temporal: Option<f32>,
    pub hqdn_chroma_temporal: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlan {
    pub id: String,
    pub kind: MediaJobKind,
    pub summary: String,
    pub inputs: Vec<String>,
    pub output_dir: Option<String>,
    pub confirmation_token: String,
}

#[derive(Debug, Clone)]
pub struct StoredMediaPlan {
    pub plan: MediaPlan,
    pub request: MediaPlanRequest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPreviewRequest {
    pub root: String,
    pub kind: MediaJobKind,
    pub max_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbeInfo {
    pub path: String,
    pub name: String,
    pub format_name: Option<String>,
    pub format_long_name: Option<String>,
    pub duration_seconds: Option<f64>,
    pub size_bytes: Option<u64>,
    pub bitrate_bps: Option<u64>,
    pub streams: Vec<MediaProbeStream>,
    pub raw_summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbeStream {
    pub index: u32,
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub profile: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<String>,
    pub pixel_format: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub duration_seconds: Option<f64>,
    pub bitrate_bps: Option<u64>,
    pub language: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRuntimeStatus {
    pub ffmpeg_version: Option<String>,
    pub path: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub download_supported: bool,
    pub message: String,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPerformanceProfile {
    pub device: MediaDeviceSummary,
    pub encoders: Vec<MediaEncoderCapability>,
    pub recommended: MediaRecommendedSettings,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDeviceSummary {
    pub cpu_name: String,
    pub cpu_physical_cores: usize,
    pub cpu_logical_cores: usize,
    pub ram_total: u64,
    pub ram_available: u64,
    pub gpus: Vec<MediaGpuSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGpuSummary {
    pub name: String,
    pub vram_bytes: Option<u64>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaEncoderCapability {
    pub encoder: VideoAv1Encoder,
    pub ffmpeg_name: String,
    pub available: bool,
    pub hardware: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaRecommendedSettings {
    pub av1_encoder: VideoAv1Encoder,
    pub av1_speed: u8,
    pub av1_crf: u8,
    pub av1_threads: usize,
    pub av1_tile_columns: u8,
    pub av1_tile_rows: u8,
    pub video_concurrency: usize,
    pub summary: String,
    pub ffmpeg_args: Vec<String>,
}
