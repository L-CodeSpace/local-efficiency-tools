/*
 * 核心职责：预览并执行文件操作。
 * 业务痛点：写入、删除和重命名必须先生成确认计划。
 * 能力边界：只处理文件操作计划和执行。
 */

use super::*;

pub fn preview_operation(
    app: &AppHandle,
    state: &AppState,
    request: FileOperationRequest,
) -> AppResult<FileOperationPlan> {
    let (kind, path, summary, risk) = match &request {
        FileOperationRequest::WriteText { path, content } => {
            if content.len() > 2 * 1024 * 1024 {
                return Err(AppError::new(
                    "content_too_large",
                    "文本写入内容超过 2MB 限制",
                ));
            }
            (
                FileOperationKind::WriteText,
                path,
                format!("保存文本文件：{path}"),
                OperationRisk::Medium,
            )
        }
        FileOperationRequest::Delete { path, recursive } => (
            FileOperationKind::Delete,
            path,
            if recursive.unwrap_or(false) {
                format!("递归删除目录或文件：{path}")
            } else {
                format!("删除文件或空目录：{path}")
            },
            OperationRisk::High,
        ),
        FileOperationRequest::Rename { path, new_name } => (
            FileOperationKind::Rename,
            path,
            format!("将 {path} 重命名为 {new_name}"),
            OperationRisk::Medium,
        ),
        FileOperationRequest::CreateFile { path } => (
            FileOperationKind::CreateFile,
            path,
            format!("创建空文件：{path}"),
            OperationRisk::Low,
        ),
        FileOperationRequest::CreateDir { path } => (
            FileOperationKind::CreateDir,
            path,
            format!("创建目录：{path}"),
            OperationRisk::Low,
        ),
    };

    ensure_allowed_path(app, state, &PathBuf::from(path))?;
    let plan = FileOperationPlan {
        id: Uuid::new_v4().to_string(),
        kind,
        target_path: path.clone(),
        summary,
        risk,
        requires_confirmation: true,
        confirmation_token: Uuid::new_v4().simple().to_string(),
        created_at: now_millis(),
    };
    state
        .file_plans
        .lock()
        .map_err(|_| AppError::fatal("file_plan_store_poisoned", "文件计划存储锁已损坏"))?
        .insert(
            plan.id.clone(),
            StoredFileOperationPlan {
                plan: plan.clone(),
                request,
            },
        );
    Ok(plan)
}

pub fn execute_operation(
    app: &AppHandle,
    state: &AppState,
    plan_id: String,
    confirmation_token: String,
) -> AppResult<Option<FileEntry>> {
    let stored = state
        .file_plans
        .lock()
        .map_err(|_| AppError::fatal("file_plan_store_poisoned", "文件计划存储锁已损坏"))?
        .remove(&plan_id)
        .ok_or_else(|| AppError::new("file_plan_not_found", "文件操作计划不存在"))?;
    if stored.plan.confirmation_token != confirmation_token {
        return Err(AppError::new(
            "confirmation_token_mismatch",
            "确认令牌不匹配",
        ));
    }

    match stored.request {
        FileOperationRequest::WriteText { path, content } => {
            let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, content)?;
            fs_guard::file_entry(&target).map(Some)
        }
        FileOperationRequest::Delete { path, recursive } => {
            let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
            if target.is_dir() {
                if recursive.unwrap_or(false) {
                    fs::remove_dir_all(&target)?;
                } else {
                    fs::remove_dir(&target).map_err(|error| {
                        AppError::new("delete_directory_not_empty", "只允许删除空目录")
                            .with_detail(error.to_string())
                    })?;
                }
            } else {
                fs::remove_file(&target)?;
            }
            Ok(None)
        }
        FileOperationRequest::Rename { path, new_name } => {
            if new_name.contains('/') || new_name.contains('\\') {
                return Err(AppError::new(
                    "invalid_file_name",
                    "新名称不能包含路径分隔符",
                ));
            }
            let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
            let parent = target
                .parent()
                .ok_or_else(|| AppError::new("invalid_path", "无法读取原路径父目录"))?;
            let next = parent.join(new_name);
            ensure_allowed_path(app, state, &next)?;
            fs::rename(&target, &next)?;
            fs_guard::file_entry(&next).map(Some)
        }
        FileOperationRequest::CreateFile { path } => {
            let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            fs_guard::file_entry(&target).map(Some)
        }
        FileOperationRequest::CreateDir { path } => {
            let target = ensure_allowed_path(app, state, &PathBuf::from(path))?;
            fs::create_dir_all(&target)?;
            fs_guard::file_entry(&target).map(Some)
        }
    }
}
