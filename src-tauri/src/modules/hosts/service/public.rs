/*
 * 核心职责：提供 hosts 管理公共 API。
 * 业务痛点：前端命令需要稳定入口，不能感知 helper 或写入细节。
 * 能力边界：只编排读取、预览和执行流程。
 */

use super::*;

pub fn hosts_path() -> String {
    hosts_file::hosts_path().to_string_lossy().to_string()
}

pub fn helper_status(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    #[cfg(target_os = "windows")]
    {
        return windows_hosts_helper::helper_status(app);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_hosts_helper::helper_status(app);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app;
        Ok(HostsHelperStatus {
            required: false,
            installed: false,
            running: false,
            token_exists: false,
            needs_repair: false,
            service_name: None,
            platform: std::env::consts::OS.to_string(),
            helper_kind: None,
            install_supported: false,
            message: "当前平台不需要 hosts helper。".to_string(),
        })
    }
}

pub fn install_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    #[cfg(target_os = "windows")]
    {
        return windows_hosts_helper::install_helper(app);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_hosts_helper::install_helper(app);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app;
        Err(AppError::new(
            "hosts_helper_unsupported",
            "当前平台不支持安装 hosts helper",
        ))
    }
}

pub fn repair_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    #[cfg(target_os = "windows")]
    {
        return windows_hosts_helper::repair_helper(app);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_hosts_helper::repair_helper(app);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app;
        Err(AppError::new(
            "hosts_helper_unsupported",
            "当前平台不支持修复 hosts helper",
        ))
    }
}

pub fn uninstall_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    #[cfg(target_os = "windows")]
    {
        return windows_hosts_helper::uninstall_helper(app);
    }

    #[cfg(target_os = "macos")]
    {
        return macos_hosts_helper::uninstall_helper(app);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = app;
        Err(AppError::new(
            "hosts_helper_unsupported",
            "当前平台不支持卸载 hosts helper",
        ))
    }
}

pub fn read_hosts() -> AppResult<Vec<HostEntry>> {
    hosts_file::read_hosts()
}

pub fn preview_change(state: &AppState, request: HostsChangeRequest) -> AppResult<HostsChangePlan> {
    validate_request(&request)?;
    let entries = hosts_file::read_hosts().unwrap_or_default();
    let action = match request.action {
        HostsChangeAction::Add => "新增",
        HostsChangeAction::Remove => "删除",
        HostsChangeAction::Toggle => "切换",
    };
    let summary = format!(
        "{action} hosts 记录：{} {}",
        request.ip.clone().unwrap_or_default(),
        request.host
    );
    let plan = HostsChangePlan {
        id: Uuid::new_v4().to_string(),
        summary,
        line_count: entries.len(),
        confirmation_token: Uuid::new_v4().simple().to_string(),
    };
    state
        .hosts_plans
        .lock()
        .map_err(|_| AppError::fatal("hosts_plan_store_poisoned", "hosts 计划存储锁已损坏"))?
        .insert(
            plan.id.clone(),
            StoredHostsChangePlan {
                plan: plan.clone(),
                request,
            },
        );
    Ok(plan)
}

pub fn execute_change(
    app: AppHandle,
    state: &AppState,
    plan_id: String,
    confirmation_token: String,
) -> AppResult<Vec<HostEntry>> {
    let stored = state
        .hosts_plans
        .lock()
        .map_err(|_| AppError::fatal("hosts_plan_store_poisoned", "hosts 计划存储锁已损坏"))?
        .remove(&plan_id)
        .ok_or_else(|| AppError::new("hosts_plan_not_found", "hosts 变更计划不存在"))?;
    if stored.plan.confirmation_token != confirmation_token {
        return Err(AppError::new(
            "confirmation_token_mismatch",
            "确认令牌不匹配",
        ));
    }
    let path = hosts_file::hosts_path();
    let content = fs::read_to_string(&path).map_err(|error| {
        AppError::new("hosts_read_failed", "读取 hosts 文件失败").with_detail(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })?;
    let next_content = apply_change(&content, &stored.request);
    write_hosts_content(&app, &path, &next_content)?;
    hosts_file::read_hosts()
}
