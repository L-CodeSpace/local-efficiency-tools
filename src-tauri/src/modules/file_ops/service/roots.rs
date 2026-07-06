/*
 * 核心职责：管理文件授权和读取入口。
 * 业务痛点：文件访问边界必须先经过授权根校验。
 * 能力边界：只处理根目录、列表和文本读取入口。
 */

use super::*;

pub fn locations() -> AppResult<FileLocations> {
    let current_dir = std::env::current_dir()?;
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| current_dir.clone());
    Ok(FileLocations {
        current_dir: current_dir.to_string_lossy().to_string(),
        executable_dir: executable_dir.to_string_lossy().to_string(),
    })
}

pub fn list_roots(app: &AppHandle, state: &AppState) -> AppResult<Vec<AuthorizedRoot>> {
    let mut roots = fs_guard::base_authorized_roots(app)?;
    roots.extend(dynamic_roots(state)?);
    roots.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(roots)
}

pub fn authorize_path(
    state: &AppState,
    path: String,
    label: Option<String>,
) -> AppResult<AuthorizedRoot> {
    let raw = PathBuf::from(&path);
    let root_path = if raw.is_file() {
        raw.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| AppError::new("invalid_path", "无法读取文件父目录"))?
    } else {
        raw
    };
    let canonical = fs_guard::canonicalize_existing(&root_path)?;
    if !canonical.is_dir() {
        return Err(AppError::new("not_a_directory", "授权根必须是目录"));
    }
    let path = canonical.to_string_lossy().to_string();
    let id = format!("user:{}", path.to_lowercase());
    let root = AuthorizedRoot {
        id: id.clone(),
        label: label.unwrap_or_else(|| {
            canonical
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "用户选择目录".to_string())
        }),
        path,
    };
    state
        .authorized_roots
        .lock()
        .map_err(|_| AppError::fatal("authorized_roots_poisoned", "授权根存储锁已损坏"))?
        .insert(id, root.clone());
    Ok(root)
}

pub(crate) fn dynamic_roots(state: &AppState) -> AppResult<Vec<AuthorizedRoot>> {
    Ok(state
        .authorized_roots
        .lock()
        .map_err(|_| AppError::fatal("authorized_roots_poisoned", "授权根存储锁已损坏"))?
        .values()
        .cloned()
        .collect())
}

pub(crate) fn ensure_allowed_path(
    app: &AppHandle,
    state: &AppState,
    path: &Path,
) -> AppResult<PathBuf> {
    fs_guard::ensure_allowed(app, path, &dynamic_roots(state)?)
}

pub fn list_dir(app: &AppHandle, state: &AppState, path: String) -> AppResult<Vec<FileEntry>> {
    let path = PathBuf::from(path);
    let allowed = ensure_allowed_path(app, state, &path)?;
    if !allowed.is_dir() {
        return Err(AppError::new("not_a_directory", "目标路径不是目录"));
    }
    let mut entries = fs::read_dir(allowed)?
        .filter_map(Result::ok)
        .take(1000)
        .filter_map(|entry| fs_guard::file_entry(&entry.path()).ok())
        .collect::<Vec<_>>();
    sort_entries(&mut entries);
    Ok(entries)
}

pub fn list_dir_recursive(
    app: &AppHandle,
    state: &AppState,
    request: FileRecursiveListRequest,
) -> AppResult<Vec<FileEntry>> {
    let root = ensure_allowed_path(app, state, &PathBuf::from(&request.path))?;
    if !root.is_dir() {
        return Err(AppError::new("not_a_directory", "目标路径不是目录"));
    }
    let extensions = request.extensions.map(|items| {
        items
            .into_iter()
            .map(|item| item.trim_start_matches('.').to_lowercase())
            .collect::<HashSet<_>>()
    });
    let mut entries = Vec::new();
    collect_entries(
        &root,
        request.max_depth.max(1).min(20),
        1,
        request.files_only.unwrap_or(false),
        extensions.as_ref(),
        &mut entries,
    )?;
    sort_entries(&mut entries);
    Ok(entries)
}

pub fn read_text(app: &AppHandle, state: &AppState, path: String) -> AppResult<String> {
    let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
    if !target.is_file() {
        return Err(AppError::new("not_a_file", "目标路径不是文件"));
    }
    let metadata = fs::metadata(&target)?;
    if metadata.len() > 2 * 1024 * 1024 {
        return Err(AppError::new(
            "file_too_large",
            "只允许编辑 2MB 以内的文本文件",
        ));
    }
    fs::read_to_string(target).map_err(|error| {
        AppError::new("text_read_failed", "读取文本文件失败").with_detail(error.to_string())
    })
}
