/*
 * 核心职责：实现批量重命名业务逻辑。
 * 业务痛点：预览、冲突检测和执行必须复用授权路径边界，避免直接操作未授权文件。
 * 能力边界：只处理批量重命名，不承担通用文件管理操作。
 */

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use tauri::AppHandle;
use uuid::Uuid;

use crate::{
    modules::{
        file_ops::service::ensure_allowed_path,
        jobs::{
            dto::JobSnapshot,
            service::{create_job, start_synthetic_job},
        },
        rename::dto::{RenameExecuteRequest, RenamePlan, RenamePreviewItem, RenamePreviewRequest},
        state::AppState,
    },
    shared::error::{AppError, AppResult},
};

pub fn preview(
    app: &AppHandle,
    state: &AppState,
    request: RenamePreviewRequest,
) -> AppResult<RenamePlan> {
    if request.pattern.trim().is_empty() {
        return Err(AppError::new("rename_pattern_empty", "查找文本不能为空"));
    }
    let root = ensure_allowed_path(app, state, &PathBuf::from(&request.root))?;
    if !root.is_dir() {
        return Err(AppError::new(
            "not_a_directory",
            "批量重命名根路径必须是目录",
        ));
    }

    let regex = if request.use_regex.unwrap_or(true) {
        Some(Regex::new(&request.pattern).map_err(|error| {
            AppError::new("rename_regex_invalid", "正则表达式无效").with_detail(error.to_string())
        })?)
    } else {
        None
    };

    let mut paths = Vec::new();
    collect_files(&root, request.max_depth.max(1).min(20), 1, &mut paths)?;
    let existing_names = paths
        .iter()
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .collect::<HashSet<_>>();
    let mut planned_names = HashSet::new();
    let mut items = Vec::new();
    let mut index = 1usize;

    for path in paths {
        let original_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let (matched, mut new_name) = next_name(&original_name, &request, regex.as_ref(), index);
        if matched {
            index += 1;
        }
        if matched && request.preserve_extension {
            preserve_extension(&path, &mut new_name);
        }
        let changed = matched && new_name != original_name;
        let mut collision = changed
            && ((existing_names.contains(&new_name) && new_name != original_name)
                || planned_names.contains(&new_name));
        let mut auto_resolved = false;
        if collision && request.auto_resolve_collision.unwrap_or(false) {
            new_name = resolve_collision(
                &new_name,
                &original_name,
                &existing_names,
                &planned_names,
                request.collision_start_index.unwrap_or(1),
            );
            collision = false;
            auto_resolved = true;
        }
        if changed && !collision {
            planned_names.insert(new_name.clone());
        }
        items.push(RenamePreviewItem {
            original_path: path.to_string_lossy().to_string(),
            original_name,
            new_name,
            selected: changed && !collision,
            collision,
            auto_resolved,
        });
    }

    let plan = RenamePlan {
        id: Uuid::new_v4().to_string(),
        root: root.to_string_lossy().to_string(),
        items,
        confirmation_token: Uuid::new_v4().simple().to_string(),
    };
    state
        .rename_plans
        .lock()
        .map_err(|_| AppError::fatal("rename_plan_store_poisoned", "重命名计划存储锁已损坏"))?
        .insert(plan.id.clone(), plan.clone());
    Ok(plan)
}

pub fn execute(
    app: AppHandle,
    state: AppState,
    request: RenameExecuteRequest,
) -> AppResult<JobSnapshot> {
    let mut plan = state
        .rename_plans
        .lock()
        .map_err(|_| AppError::fatal("rename_plan_store_poisoned", "重命名计划存储锁已损坏"))?
        .remove(&request.plan_id)
        .ok_or_else(|| AppError::new("rename_plan_not_found", "重命名计划不存在"))?;
    if plan.confirmation_token != request.confirmation_token {
        return Err(AppError::new(
            "confirmation_token_mismatch",
            "确认令牌不匹配",
        ));
    }
    if let Some(selected_paths) = request.selected_original_paths {
        let selected = selected_paths.into_iter().collect::<HashSet<_>>();
        for item in &mut plan.items {
            item.selected = selected.contains(&item.original_path);
        }
    }

    let selected_count = plan.items.iter().filter(|item| item.selected).count();
    for item in plan.items.iter().filter(|item| item.selected) {
        let source = ensure_allowed_path(&app, &state, &PathBuf::from(&item.original_path))?;
        let target = source
            .parent()
            .ok_or_else(|| AppError::new("invalid_path", "无法读取父目录"))?
            .join(&item.new_name);
        ensure_allowed_path(&app, &state, &target)?;
        fs::rename(source, target)?;
    }

    let job = create_job(
        &state,
        "batchRename",
        format!("批量重命名 {selected_count} 个文件"),
    )?;
    start_synthetic_job(
        app,
        state,
        job,
        format!("已完成 {selected_count} 个文件重命名。"),
    )
}

fn next_name(
    original_name: &str,
    request: &RenamePreviewRequest,
    regex: Option<&Regex>,
    index: usize,
) -> (bool, String) {
    let replaced = if let Some(regex) = regex {
        if !regex.is_match(original_name) {
            return (false, original_name.to_string());
        }
        regex
            .replace_all(original_name, request.replacement.as_str())
            .to_string()
    } else if original_name.contains(&request.pattern) {
        original_name.replace(&request.pattern, &request.replacement)
    } else {
        return (false, original_name.to_string());
    };
    (true, replaced.replace("$INDEX", &index.to_string()))
}

fn preserve_extension(path: &Path, new_name: &mut String) {
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        let suffix = format!(".{extension}");
        if !new_name.ends_with(&suffix) {
            new_name.push_str(&suffix);
        }
    }
}

fn resolve_collision(
    candidate: &str,
    original: &str,
    existing_names: &HashSet<String>,
    planned_names: &HashSet<String>,
    start: usize,
) -> String {
    let (base, extension) = split_extension(candidate);
    let mut counter = start;
    loop {
        let next = format!("{base}-{counter}{extension}");
        if !planned_names.contains(&next) && (!existing_names.contains(&next) || next == original) {
            return next;
        }
        counter += 1;
    }
}

fn split_extension(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(index) if index > 0 => (&name[..index], &name[index..]),
        _ => (name, ""),
    }
}

fn collect_files(
    root: &Path,
    max_depth: usize,
    depth: usize,
    out: &mut Vec<PathBuf>,
) -> AppResult<()> {
    if depth > max_depth {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, max_depth, depth + 1, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}
