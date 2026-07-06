/*
 * 核心职责：递归扫描和排序文件条目。
 * 业务痛点：目录扫描需要统一过滤和排序规则。
 * 能力边界：只提供扫描辅助函数。
 */

use super::*;

pub(super) fn collect_entries(
    root: &Path,
    max_depth: usize,
    depth: usize,
    files_only: bool,
    extensions: Option<&HashSet<String>>,
    out: &mut Vec<FileEntry>,
) -> AppResult<()> {
    if depth > max_depth {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            if !files_only {
                out.push(fs_guard::file_entry(&path)?);
            }
            collect_entries(&path, max_depth, depth + 1, files_only, extensions, out)?;
        } else if extension_matches(&path, extensions) {
            out.push(fs_guard::file_entry(&path)?);
        }
    }
    Ok(())
}

pub(super) fn extension_matches(path: &Path, extensions: Option<&HashSet<String>>) -> bool {
    let Some(extensions) = extensions else {
        return true;
    };
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| extensions.contains(&value.to_lowercase()))
        .unwrap_or(false)
}

pub(super) fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
}
