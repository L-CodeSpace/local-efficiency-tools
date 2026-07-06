/*
 * 核心职责：下载并安装 FFmpeg 运行时。
 * 业务痛点：下载、校验、解压和替换运行时必须保持原子化。
 * 能力边界：只处理运行时资产下载与安装。
 */

use super::*;

#[derive(Clone, Copy)]
pub(super) struct FfmpegAsset {
    pub(super) filename: &'static str,
    pub(super) url: &'static str,
    pub(super) sha256: &'static str,
    pub(super) source_name: &'static str,
    pub(super) source_url: &'static str,
}

pub(super) fn current_ffmpeg_asset() -> AppResult<FfmpegAsset> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok(FfmpegAsset {
            filename: "ffmpeg-windows-x64.zip",
            url:
                "https://github.com/Tyrrrz/FFmpegBin/releases/download/8.1.1/ffmpeg-windows-x64.zip",
            sha256: "9964d9fcd82889c867c3742bdcd541b8565dc3508f6ed71daaeece77dafce41c",
            source_name: "Tyrrrz/FFmpegBin 8.1.1 Windows x64",
            source_url: "https://github.com/Tyrrrz/FFmpegBin/releases/tag/8.1.1",
        });
    }

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return Ok(FfmpegAsset {
            filename: "ffmpeg-windows-arm64.zip",
            url: "https://github.com/Tyrrrz/FFmpegBin/releases/download/8.1.1/ffmpeg-windows-arm64.zip",
            sha256: "f8c79d6e7099e4eec251fc7fd9c2903c5e18010acf6da8af8d7f37f24e9e45ea",
            source_name: "Tyrrrz/FFmpegBin 8.1.1 Windows ARM64",
            source_url: "https://github.com/Tyrrrz/FFmpegBin/releases/tag/8.1.1",
        });
    }

    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        return Ok(FfmpegAsset {
            filename: "ffmpeg-windows-x86.zip",
            url:
                "https://github.com/Tyrrrz/FFmpegBin/releases/download/8.1.1/ffmpeg-windows-x86.zip",
            sha256: "bca0ae5832bf5055d28479fba5949a17743408415017b3557d8088b9b0bd3ce6",
            source_name: "Tyrrrz/FFmpegBin 8.1.1 Windows x86",
            source_url: "https://github.com/Tyrrrz/FFmpegBin/releases/tag/8.1.1",
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(FfmpegAsset {
            filename: "ffmpeg-osx-x64.zip",
            url: "https://github.com/Tyrrrz/FFmpegBin/releases/download/8.1.1/ffmpeg-osx-x64.zip",
            sha256: "36723a7be2233e17a1f8de7998392228a2f0982453635463e2758bb1f7d5eead",
            source_name: "Tyrrrz/FFmpegBin 8.1.1 macOS x64",
            source_url: "https://github.com/Tyrrrz/FFmpegBin/releases/tag/8.1.1",
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(FfmpegAsset {
            filename: "ffmpeg-osx-arm64.zip",
            url: "https://github.com/Tyrrrz/FFmpegBin/releases/download/8.1.1/ffmpeg-osx-arm64.zip",
            sha256: "0bae4f67393eb210ad99e8aa8d787cd27f238104709ed5d5a8fdfc7d104d17f6",
            source_name: "Tyrrrz/FFmpegBin 8.1.1 macOS ARM64",
            source_url: "https://github.com/Tyrrrz/FFmpegBin/releases/tag/8.1.1",
        });
    }

    #[allow(unreachable_code)]
    Err(AppError::new(
        "media_runtime_unsupported",
        "当前平台暂未配置 FFmpeg 自动下载源",
    ))
}

pub(super) fn ffmpeg_source_metadata() -> (Option<String>, Option<String>, bool) {
    match current_ffmpeg_asset() {
        Ok(asset) => (
            Some(asset.source_name.to_string()),
            Some(asset.source_url.to_string()),
            true,
        ),
        Err(_) => (None, None, false),
    }
}

pub(super) fn download_with_sha256(
    client: &Client,
    app: &AppHandle,
    label: &str,
    url: &str,
    path: &Path,
) -> AppResult<String> {
    let mut response = client.get(url).send().map_err(|error| {
        AppError::new("media_runtime_download_failed", "下载 FFmpeg 失败")
            .with_detail(error.to_string())
    })?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "media_runtime_download_failed",
            format!("下载 FFmpeg 失败: HTTP {}", response.status()),
        ));
    }

    let mut file = fs::File::create(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32768];
    let mut downloaded = 0u64;
    let mut progress = observability::DownloadProgress::new(label, response.content_length());
    loop {
        let count = response.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        file.write_all(&buffer[..count])?;
        downloaded += count as u64;
        progress.record(app, downloaded);
    }
    progress.finish(app, downloaded);
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn install_ffmpeg_runtime(
    archive_path: &Path,
    runtime_dir: &Path,
    staging_dir: &Path,
    backup_dir: &Path,
) -> AppResult<()> {
    cleanup_path(staging_dir)?;
    cleanup_path(backup_dir)?;
    if let Err(error) = extract_ffmpeg_runtime(archive_path, staging_dir) {
        let _ = cleanup_path(staging_dir);
        return Err(error);
    }
    replace_runtime_dir(runtime_dir, staging_dir, backup_dir)
}

pub(super) fn extract_ffmpeg_runtime(archive_path: &Path, target_dir: &Path) -> AppResult<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        AppError::new("media_runtime_extract_failed", "解压 FFmpeg ZIP 失败")
            .with_detail(error.to_string())
    })?;
    fs::create_dir_all(target_dir)?;
    let mut extracted_files = 0usize;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            AppError::new("media_runtime_extract_failed", "读取 FFmpeg ZIP 失败")
                .with_detail(error.to_string())
        })?;

        let Some(relative_path) = safe_zip_entry_path(entry.name()) else {
            continue;
        };
        let output_path = target_dir.join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&output_path)?;
        std::io::copy(&mut entry, &mut out)?;
        extracted_files += 1;

        #[cfg(unix)]
        if is_ffmpeg_runtime_tool(&output_path) {
            set_executable_permission(&output_path)?;
        }
    }

    if extracted_files == 0 {
        return Err(AppError::new(
            "media_runtime_extract_failed",
            "FFmpeg ZIP 中未找到可解压文件",
        ));
    }

    let ffmpeg_path = target_dir.join(ffmpeg_executable_name());
    if !ffmpeg_path.is_file() {
        cleanup_path(target_dir)?;
        return Err(AppError::new(
            "media_runtime_extract_failed",
            "FFmpeg ZIP 中未找到 ffmpeg 可执行文件",
        ));
    }

    Ok(())
}

pub(super) fn replace_runtime_dir(
    runtime_dir: &Path,
    staging_dir: &Path,
    backup_dir: &Path,
) -> AppResult<()> {
    let had_runtime = runtime_dir.exists();
    if had_runtime {
        fs::rename(runtime_dir, backup_dir).map_err(|error| {
            AppError::new("media_runtime_install_failed", "备份旧 FFmpeg runtime 失败")
                .with_detail(error.to_string())
        })?;
    }

    match fs::rename(staging_dir, runtime_dir) {
        Ok(()) => {
            let _ = cleanup_path(backup_dir);
            Ok(())
        }
        Err(error) => {
            let restore_error = if had_runtime && backup_dir.exists() && !runtime_dir.exists() {
                fs::rename(backup_dir, runtime_dir).err()
            } else {
                None
            };
            let _ = cleanup_path(staging_dir);
            let detail = match restore_error {
                Some(restore_error) => {
                    format!("{}；回滚旧 FFmpeg runtime 失败: {}", error, restore_error)
                }
                None => error.to_string(),
            };
            Err(
                AppError::new("media_runtime_install_failed", "安装 FFmpeg runtime 失败")
                    .with_detail(detail),
            )
        }
    }
}

pub(super) fn safe_zip_entry_path(name: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");
    let mut path = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

pub(super) fn cleanup_path(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub(super) fn ffmpeg_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

pub(super) fn ffprobe_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffprobe.exe"
    } else {
        "ffprobe"
    }
}

#[cfg(unix)]
pub(super) fn is_ffmpeg_runtime_tool(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some("ffmpeg" | "ffplay" | "ffprobe")
    )
}

#[cfg(unix)]
pub(super) fn set_executable_permission(path: &Path) -> AppResult<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
