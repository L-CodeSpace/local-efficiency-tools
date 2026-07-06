/*
 * 核心职责：收集媒体输入文件。
 * 业务痛点：文件夹预览需要统一扩展名过滤。
 * 能力边界：只处理媒体扩展名和递归收集。
 */

use super::*;

pub(super) const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "ico", "webp", "heic", "heif", "avif",
    "svg", "cr2", "nef", "arw",
];

pub(super) const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mkv", "avi", "webm", "wmv", "m4v", "ts", "m2ts", "vob", "rmvb", "rm",
];

pub(super) fn collect_media_files(
    root: &Path,
    max_depth: usize,
    depth: usize,
    extensions: &[&str],
    out: &mut Vec<PathBuf>,
) -> AppResult<()> {
    if depth > max_depth {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_media_files(&path, max_depth, depth + 1, extensions, out)?;
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| extensions.contains(&value.to_lowercase().as_str()))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}
