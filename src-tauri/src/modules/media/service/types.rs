/*
 * 核心职责：定义媒体任务内部模型。
 * 业务痛点：任务执行需要共享轻量内部状态，但不应暴露到领域层。
 * 能力边界：只保存本模块内部结构和枚举。
 */

use super::*;

#[derive(Clone)]
pub(super) struct MediaWorkItem {
    pub(super) input: PathBuf,
    pub(super) output: PathBuf,
    pub(super) label: &'static str,
    pub(super) kind: MediaWorkKind,
}

#[derive(Clone)]
pub(super) enum MediaWorkKind {
    ImageWebp {
        quality: u8,
        corner_radius: Option<u32>,
    },
    ImageAvif {
        quality: u8,
        corner_radius: Option<u32>,
    },
    VideoAnimatedWebp {
        quality: u8,
        corner_radius: Option<u32>,
        hqdn3d: Option<Hqdn3d>,
    },
    VideoAv1 {
        with_audio: bool,
        encoder: VideoAv1Encoder,
        speed: u8,
        crf: u8,
        threads: usize,
        tile_columns: u8,
        tile_rows: u8,
        concurrency: usize,
        hqdn3d: Option<Hqdn3d>,
    },
    AudioMp3,
}

#[derive(Clone, Copy)]
pub(super) struct Hqdn3d {
    pub(super) luma_spatial: f32,
    pub(super) chroma_spatial: f32,
    pub(super) luma_temporal: f32,
    pub(super) chroma_temporal: f32,
}

#[derive(Clone, Copy)]
pub(super) enum Av1Encoder {
    Nvenc,
    SvtAv1,
    AomAv1,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Av1EncoderSet {
    pub(super) nvenc: bool,
    pub(super) svt: bool,
    pub(super) aom: bool,
}

pub(super) struct MediaFailure {
    pub(super) output: String,
    pub(super) detail: String,
}

pub(super) enum FfmpegRunOutcome {
    Succeeded,
    Failed(String),
    Cancelled,
}

#[derive(Clone)]
pub(super) struct IndexedMediaWorkItem {
    pub(super) index: usize,
    pub(super) item: MediaWorkItem,
}

pub(super) struct VideoWorkGroup {
    pub(super) id: String,
    pub(super) input: PathBuf,
    pub(super) items: Vec<IndexedMediaWorkItem>,
}

#[derive(Clone, Copy, Default)]
pub(super) struct FfmpegProgress {
    pub(super) frame: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeOutput {
    pub(super) streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FfprobeStream {
    pub(super) nb_read_frames: Option<String>,
    pub(super) nb_frames: Option<String>,
    pub(super) avg_frame_rate: Option<String>,
    pub(super) r_frame_rate: Option<String>,
    pub(super) duration: Option<String>,
}
