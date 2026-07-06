/*
 * 核心职责：创建媒体计划并预览输入。
 * 业务痛点：媒体入口逻辑需要和执行细节隔离，避免计划校验被转码流程淹没。
 * 能力边界：只处理计划、启动和输入预览，不拼装 FFmpeg 细节。
 */

use super::*;

pub fn create_plan(state: &AppState, request: MediaPlanRequest) -> AppResult<MediaPlan> {
    if request.inputs.is_empty() {
        return Err(AppError::new("media_inputs_empty", "至少需要一个输入路径"));
    }
    if request.inputs.len() > 500 {
        return Err(AppError::new(
            "media_inputs_too_many",
            "单个媒体计划最多允许 500 个输入",
        ));
    }
    if matches!(request.kind, MediaJobKind::VideoTranscode)
        && request
            .video_targets
            .as_ref()
            .map(|targets| targets.is_empty())
            .unwrap_or(true)
    {
        return Err(AppError::new(
            "media_targets_empty",
            "至少需要选择一个视频输出目标",
        ));
    }

    let title = match &request.kind {
        MediaJobKind::ImageCompression => "图片处理",
        MediaJobKind::VideoTranscode => "视频处理",
    };
    let summary = format!(
        "{title}计划：{} 个输入，输出目录 {}",
        request.inputs.len(),
        request.output_dir.as_deref().unwrap_or("自动")
    );
    let plan = MediaPlan {
        id: Uuid::new_v4().to_string(),
        kind: request.kind.clone(),
        summary,
        inputs: request.inputs.clone(),
        output_dir: request.output_dir.clone(),
        confirmation_token: Uuid::new_v4().simple().to_string(),
    };
    state
        .media_plans
        .lock()
        .map_err(|_| AppError::fatal("media_plan_store_poisoned", "媒体计划存储锁已损坏"))?
        .insert(
            plan.id.clone(),
            StoredMediaPlan {
                plan: plan.clone(),
                request,
            },
        );
    Ok(plan)
}

pub fn start_job(
    app: AppHandle,
    state: AppState,
    plan_id: String,
    confirmation_token: String,
) -> AppResult<JobSnapshot> {
    let stored = state
        .media_plans
        .lock()
        .map_err(|_| AppError::fatal("media_plan_store_poisoned", "媒体计划存储锁已损坏"))?
        .remove(&plan_id)
        .ok_or_else(|| AppError::new("media_plan_not_found", "媒体计划不存在"))?;
    if stored.plan.confirmation_token != confirmation_token {
        return Err(AppError::new(
            "confirmation_token_mismatch",
            "确认令牌不匹配",
        ));
    }

    let ffmpeg_path = ffmpeg_path_for_execution(&app)?;
    let work = build_media_work(&app, &state, &stored.request)?;
    let job_kind = stored.request.kind.clone();

    let (kind, title) = match &stored.request.kind {
        MediaJobKind::ImageCompression => ("imageCompression", "图片 Pipeline"),
        MediaJobKind::VideoTranscode => ("videoTranscode", "视频 Pipeline"),
    };
    let job = create_job(&state, kind, format!("{title} · {} 个产物", work.len()))?;
    let job = update_job(&app, &state, &job.id, |snapshot| {
        snapshot.status = JobStatus::Running;
        snapshot.message = "任务开始执行。".to_string();
    })?;

    let job_id = job.id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        execute_media_job(app, state, job_id, ffmpeg_path, job_kind, work);
    });

    Ok(job)
}

pub fn preview_inputs(
    app: &AppHandle,
    state: &AppState,
    request: MediaPreviewRequest,
) -> AppResult<Vec<String>> {
    let root = ensure_allowed_path(app, state, &PathBuf::from(&request.root))?;
    if !root.is_dir() {
        return Err(AppError::new("not_a_directory", "媒体预览根路径必须是目录"));
    }
    let extensions = match request.kind {
        MediaJobKind::ImageCompression => IMAGE_EXTENSIONS,
        MediaJobKind::VideoTranscode => VIDEO_EXTENSIONS,
    };
    let mut paths = Vec::new();
    collect_media_files(
        &root,
        request.max_depth.max(1).min(20),
        1,
        extensions,
        &mut paths,
    )?;
    paths.sort();
    Ok(paths
        .into_iter()
        .take(500)
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}
