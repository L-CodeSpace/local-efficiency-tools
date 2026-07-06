/*
 * 核心职责：提供跨模块文件访问安全守卫。
 * 业务痛点：文件管理和媒体处理都必须限制在授权根目录内，避免任意路径访问。
 * 能力边界：只做路径授权和文件元数据转换，不执行具体业务操作。
 */

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use tauri::{AppHandle, Manager};

use crate::{
    modules::file_ops::dto::{AuthorizedRoot, FileEntry},
    shared::error::{AppError, AppResult},
};

pub fn base_authorized_roots(app: &AppHandle) -> AppResult<Vec<AuthorizedRoot>> {
    let current_dir = std::env::current_dir()?;
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        AppError::new("app_data_dir_unavailable", "无法读取应用数据目录")
            .with_detail(error.to_string())
    })?;
    fs::create_dir_all(&app_data_dir)?;

    Ok(vec![
        AuthorizedRoot {
            id: "currentDir".to_string(),
            label: "当前工作目录".to_string(),
            path: current_dir.to_string_lossy().to_string(),
        },
        AuthorizedRoot {
            id: "appData".to_string(),
            label: "应用数据目录".to_string(),
            path: app_data_dir.to_string_lossy().to_string(),
        },
    ])
}

pub fn ensure_allowed(
    app: &AppHandle,
    path: &Path,
    additional_roots: &[AuthorizedRoot],
) -> AppResult<PathBuf> {
    let candidate = canonical_candidate(path)?;
    let mut roots = base_authorized_roots(app)?;
    roots.extend(additional_roots.iter().cloned());
    let allowed = roots.iter().any(|root| {
        fs::canonicalize(&root.path)
            .map(|root_path| candidate.starts_with(root_path))
            .unwrap_or(false)
    });
    if allowed {
        Ok(candidate)
    } else {
        Err(AppError::new("path_not_authorized", "路径不在授权根目录内")
            .with_detail(path.to_string_lossy().to_string()))
    }
}

pub fn canonicalize_existing(path: &Path) -> AppResult<PathBuf> {
    fs::canonicalize(path).map_err(AppError::from)
}

pub fn file_entry(path: &Path) -> AppResult<FileEntry> {
    let metadata = fs::metadata(path)?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    Ok(FileEntry {
        name: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        path: path.to_string_lossy().to_string(),
        parent: path
            .parent()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default(),
        is_dir: metadata.is_dir(),
        size: if metadata.is_file() {
            metadata.len()
        } else {
            0
        },
        modified_at,
        readonly: metadata.permissions().readonly(),
    })
}

fn canonical_candidate(path: &Path) -> AppResult<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).map_err(AppError::from);
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new("invalid_path", "路径缺少父目录"))?;
    let canonical_parent = fs::canonicalize(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| AppError::new("invalid_path", "路径缺少文件名"))?;
    Ok(canonical_parent.join(name))
}
