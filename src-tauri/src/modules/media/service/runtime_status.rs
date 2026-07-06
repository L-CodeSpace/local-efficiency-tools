/*
 * 核心职责：提供媒体运行时状态和下载入口。
 * 业务痛点：运行时状态是 controller API，不能和路径解析、滤镜参数等底层 helper 混在一起。
 * 能力边界：只编排 FFmpeg 运行时状态、下载和安装后的状态刷新。
 */

use super::*;
pub fn runtime_status() -> MediaRuntimeStatus {
    runtime_status_for_path(resolve_executable_path("ffmpeg"), "检测到系统 FFmpeg。")
        .unwrap_or_else(|| {
            unavailable_runtime_status(
                "未检测到系统 FFmpeg；当前执行器会以 Job Pipeline 骨架运行。",
            )
        })
}

pub fn runtime_status_with_app(app: &AppHandle) -> AppResult<MediaRuntimeStatus> {
    let local_path = ffmpeg_binary_path(app)?;
    if let Some(status) =
        runtime_status_for_path(executable_file(&local_path), "检测到应用内 FFmpeg。")
    {
        return Ok(status);
    }
    Ok(runtime_status())
}

pub fn download_runtime(app: &AppHandle) -> AppResult<MediaRuntimeStatus> {
    match download_runtime_inner(app) {
        Ok(status) => Ok(status),
        Err(error) => {
            observability::emit_error(app, format!("FFmpeg 运行时下载失败: {}", error));
            Err(error)
        }
    }
}

pub(super) fn download_runtime_inner(app: &AppHandle) -> AppResult<MediaRuntimeStatus> {
    let asset = current_ffmpeg_asset()?;
    let app_dir = app_ffmpeg_dir(app)?;
    let runtime_dir = app_dir.join("runtime");
    let staging_dir = app_dir.join("runtime.download");
    let backup_dir = app_dir.join("runtime.backup");
    let archive_path = app_dir.join(format!("{}.download", asset.filename));
    let client = Client::new();

    observability::emit_info(
        app,
        format!(
            "开始下载 FFmpeg: {} · {} ({})",
            asset.source_name, asset.filename, asset.source_url
        ),
    );
    let actual_hash = download_with_sha256(&client, app, "FFmpeg", asset.url, &archive_path)?;
    if actual_hash != asset.sha256 {
        let _ = fs::remove_file(&archive_path);
        return Err(AppError::new(
            "media_runtime_hash_mismatch",
            format!(
                "FFmpeg SHA-256 校验失败，期望 {}，实际 {}",
                asset.sha256, actual_hash
            ),
        ));
    }

    observability::emit_info(app, "FFmpeg 下载校验通过，开始解压。");
    if let Err(error) =
        install_ffmpeg_runtime(&archive_path, &runtime_dir, &staging_dir, &backup_dir)
    {
        let _ = fs::remove_file(&archive_path);
        return Err(error);
    }
    let _ = fs::remove_file(&archive_path);
    observability::emit_info(app, "FFmpeg 安装完成。");

    runtime_status_with_app(app)
}

pub(super) fn runtime_status_for_path(
    path: Option<PathBuf>,
    message: &str,
) -> Option<MediaRuntimeStatus> {
    let mut command = match path.as_ref() {
        Some(path) => Command::new(path),
        None => Command::new("ffmpeg"),
    };

    let output = command.arg("-version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("ffmpeg available")
        .to_string();
    let (source_name, source_url, download_supported) = ffmpeg_source_metadata();
    Some(MediaRuntimeStatus {
        ffmpeg_version: Some(first_line),
        path: path.map(|path| display_path(&path)),
        source_name,
        source_url,
        download_supported,
        message: message.to_string(),
        ready: true,
    })
}

pub(super) fn unavailable_runtime_status(message: &str) -> MediaRuntimeStatus {
    let (source_name, source_url, download_supported) = ffmpeg_source_metadata();
    MediaRuntimeStatus {
        ffmpeg_version: None,
        path: None,
        source_name,
        source_url,
        download_supported,
        message: message.to_string(),
        ready: false,
    }
}
