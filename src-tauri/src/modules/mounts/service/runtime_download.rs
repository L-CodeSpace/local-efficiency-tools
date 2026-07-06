/*
 * 核心职责：下载并安装 rclone 运行时。
 * 业务痛点：固定版本下载、校验和解压必须和业务配置解耦。
 * 能力边界：只处理 rclone runtime 安装。
 */

use super::runtime::{RcloneAsset, RCLONE_VERSION};
use super::*;
use super::{
    normalize::hidden_command,
    storage::{rclone_binary_path, rclone_runtime_dir},
};

pub(super) fn ensure_rclone(app: &AppHandle) -> AppResult<PathBuf> {
    let path = rclone_binary_path(app)?;
    if path.exists() {
        if let Ok(version) = get_rclone_version(&path) {
            if version.contains(RCLONE_VERSION) {
                observability::emit_info(app, format!("rclone 已就绪: {}", version));
                return Ok(path);
            }
            observability::emit_info(app, format!("rclone 版本不匹配，准备重新下载: {}", version));
        }
        let _ = fs::remove_file(&path);
    }
    download_rclone(app)?;
    Ok(path)
}

pub(super) fn download_rclone(app: &AppHandle) -> AppResult<()> {
    match download_rclone_inner(app) {
        Ok(()) => Ok(()),
        Err(error) => {
            observability::emit_error(app, format!("rclone 下载失败: {}", error));
            Err(error)
        }
    }
}

pub(super) fn download_rclone_inner(app: &AppHandle) -> AppResult<()> {
    let asset = current_asset()?;
    let dir = rclone_runtime_dir(app)?;
    fs::create_dir_all(&dir)?;
    let zip_path = dir.join(format!("{}.download", asset.filename));
    let bin_path = rclone_binary_path(app)?;
    let bin_tmp = bin_path.with_extension("download");

    observability::emit_info(
        app,
        format!("开始下载 rclone {}: {}", RCLONE_VERSION, asset.filename),
    );
    let mut response = Client::new().get(asset.url).send().map_err(|error| {
        AppError::new("mount_runtime_download_failed", "下载 rclone 失败")
            .with_detail(error.to_string())
    })?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "mount_runtime_download_failed",
            format!("下载 rclone 失败: HTTP {}", response.status()),
        ));
    }

    let mut file = fs::File::create(&zip_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32768];
    let mut downloaded = 0u64;
    let mut progress = observability::DownloadProgress::new("rclone", response.content_length());

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
    drop(file);

    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != asset.sha256 {
        let _ = fs::remove_file(&zip_path);
        return Err(AppError::new(
            "mount_runtime_hash_mismatch",
            format!(
                "rclone SHA-256 校验失败，期望 {}，实际 {}",
                asset.sha256, actual_hash
            ),
        ));
    }
    observability::emit_info(app, "rclone 下载校验通过，开始解压。");

    let zip_file = fs::File::open(&zip_path)?;
    let mut archive = ZipArchive::new(zip_file).map_err(|error| {
        AppError::new("mount_runtime_extract_failed", "解压 rclone ZIP 失败")
            .with_detail(error.to_string())
    })?;
    let exe_name = if cfg!(target_os = "windows") {
        "rclone.exe"
    } else {
        "rclone"
    };
    let mut found = false;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| {
            AppError::new("mount_runtime_extract_failed", "读取 rclone ZIP 失败")
                .with_detail(error.to_string())
        })?;
        if file.name().ends_with(exe_name) {
            let mut out = fs::File::create(&bin_tmp)?;
            std::io::copy(&mut file, &mut out)?;
            found = true;
            break;
        }
    }
    if !found {
        let _ = fs::remove_file(&zip_path);
        return Err(AppError::new(
            "mount_runtime_extract_failed",
            "rclone ZIP 中未找到可执行文件",
        ));
    }

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&bin_tmp)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin_tmp, permissions)?;
    }

    if bin_path.exists() {
        let _ = fs::remove_file(&bin_path);
    }
    fs::rename(&bin_tmp, &bin_path)?;
    let _ = fs::remove_file(&zip_path);
    observability::emit_info(app, "rclone 安装完成。");
    Ok(())
}

pub(super) fn get_rclone_version(path: &Path) -> AppResult<String> {
    let output = hidden_command(path).arg("version").output()?;
    if !output.status.success() {
        return Err(AppError::new(
            "mount_runtime_version_failed",
            "执行 rclone version 失败",
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("").trim().to_string())
}

pub(super) fn current_asset() -> AppResult<RcloneAsset> {
    #[cfg(target_os = "windows")]
    {
        return Ok(RcloneAsset {
            filename: "rclone-v1.74.3-windows-amd64.zip",
            url: "https://github.com/rclone/rclone/releases/download/v1.74.3/rclone-v1.74.3-windows-amd64.zip",
            sha256: "ecb0ed9006e0d1a693757007716a11dab6c2cde6dac3f2fd87da962eaa73d11d",
            source_name: "rclone/rclone v1.74.3",
            source_url: "https://github.com/rclone/rclone/releases/tag/v1.74.3",
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok(RcloneAsset {
            filename: "rclone-v1.74.3-osx-amd64.zip",
            url: "https://github.com/rclone/rclone/releases/download/v1.74.3/rclone-v1.74.3-osx-amd64.zip",
            sha256: "417cabd402d57806d597bd0ba8fb33a434ca8c2a1a5aa98de5a0bd4b52b39202",
            source_name: "rclone/rclone v1.74.3",
            source_url: "https://github.com/rclone/rclone/releases/tag/v1.74.3",
        });
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok(RcloneAsset {
            filename: "rclone-v1.74.3-osx-arm64.zip",
            url: "https://github.com/rclone/rclone/releases/download/v1.74.3/rclone-v1.74.3-osx-arm64.zip",
            sha256: "33a435ab17023b686918ce9a3975aceb75fe1796c694f38f1993024be1f063f5",
            source_name: "rclone/rclone v1.74.3",
            source_url: "https://github.com/rclone/rclone/releases/tag/v1.74.3",
        });
    }

    #[allow(unreachable_code)]
    Err(AppError::new(
        "mount_runtime_unsupported",
        "当前平台没有固定的 rclone 发行包",
    ))
}
