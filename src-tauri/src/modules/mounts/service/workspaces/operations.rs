/*
 * 核心职责：创建、启停、刷新、修复和删除挂载工作区。
 * 能力边界：通过 lifecycle 管理会话，通过 validation 处理边界规则。
 */

use super::super::*;
use super::super::{
    advanced_options::normalize_advanced_options,
    connection_probe::probe_connection,
    ftp_combine, native_smb,
    v2_storage::{load_mount_store, save_mount_store},
};
use super::{
    lifecycle::{
        hydrate_workspace_status, prune_exited_sessions, start_workspace_session,
        stop_workspace_session,
    },
    operation_lock,
    validation::{
        assign_local_targets, connection_by_id, ensure_unique_bindings, normalize_remote_path,
        trim_option,
    },
};
use crate::modules::state::MountSession;

pub fn list_workspaces(app: &AppHandle, state: &AppState) -> AppResult<Vec<MountWorkspace>> {
    prune_exited_sessions(app, state);
    let mut workspaces = load_mount_store(app)?.workspaces;
    for workspace in &mut workspaces {
        hydrate_workspace_status(state, workspace);
    }
    Ok(workspaces)
}

pub fn create_workspace(
    app: &AppHandle,
    state: &AppState,
    input: MountWorkspaceInput,
) -> AppResult<MountWorkspace> {
    let _operation = operation_lock()?;
    let mut store = load_mount_store(app)?;
    let connection = connection_by_id(&store, &input.connection_id)?.clone();
    if input.bindings.is_empty() {
        return Err(AppError::new(
            "mount_workspace_empty",
            "请至少选择一个远端目录",
        ));
    }
    let existing_index = input.id.as_ref().and_then(|id| {
        store
            .workspaces
            .iter()
            .position(|workspace| &workspace.id == id)
    });
    let workspace_id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());
    ensure_unique_bindings(&store, &input.connection_id, &workspace_id, &input.bindings)?;
    let effective_transport = input
        .effective_transport
        .or(probe_connection(app, &connection, &store)?.recommended_transport)
        .ok_or_else(|| {
            AppError::new(
                "mount_transport_unavailable",
                "SMB 和 FTP 均不可用，无法创建挂载工作区",
            )
        })?;
    let now = super::super::normalize::now_millis();
    let existing = existing_index.map(|index| store.workspaces[index].clone());
    let mut workspace = MountWorkspace {
        id: workspace_id,
        connection_id: input.connection_id,
        name: input.name.trim().to_string(),
        bindings: input
            .bindings
            .into_iter()
            .map(|binding| RemoteBinding {
                id: Uuid::new_v4().simple().to_string(),
                name: binding.name.trim().to_string(),
                remote_path: normalize_remote_path(&binding.remote_path),
                drive_letter: trim_option(binding.drive_letter),
                mount_point: trim_option(binding.mount_point),
                accessible: true,
                error: None,
            })
            .collect(),
        drive_letter: trim_option(input.drive_letter),
        mount_point: trim_option(input.mount_point),
        advanced_options: normalize_advanced_options(input.advanced_options, cfg!(windows))?,
        enabled: input.enabled.unwrap_or(false),
        created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
        updated_at: now,
        effective_transport: Some(effective_transport),
        mounted: false,
        status: MountStatus::Disabled,
        error: None,
    };
    assign_local_targets(app, &store, &connection, &mut workspace)?;
    if let Some(index) = existing_index {
        stop_workspace_session(app, state, &workspace.id)?;
        store.workspaces[index] = workspace.clone();
    } else {
        store.workspaces.push(workspace.clone());
    }
    save_mount_store(app, &store)?;
    if workspace.enabled {
        if let Err(error) = start_workspace_session(app, state, &store, &connection, &workspace) {
            workspace.enabled = false;
            workspace.error = Some(error.message.clone());
            replace_workspace(app, &workspace)?;
            return Err(error);
        }
    }
    hydrate_workspace_status(state, &mut workspace);
    Ok(workspace)
}

pub fn delete_workspace(app: &AppHandle, state: &AppState, id: &str) -> AppResult<()> {
    let _operation = operation_lock()?;
    stop_workspace_session(app, state, id)?;
    let mut store = load_mount_store(app)?;
    if !store.workspaces.iter().any(|workspace| workspace.id == id) {
        return Err(AppError::new(
            "mount_workspace_not_found",
            "未找到挂载工作区",
        ));
    }
    store.workspaces.retain(|workspace| workspace.id != id);
    save_mount_store(app, &store)
}

pub fn set_workspace_enabled(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    enabled: bool,
) -> AppResult<MountWorkspace> {
    let _operation = operation_lock()?;
    let mut store = load_mount_store(app)?;
    let index = store
        .workspaces
        .iter()
        .position(|workspace| workspace.id == id)
        .ok_or_else(|| AppError::new("mount_workspace_not_found", "未找到挂载工作区"))?;
    let connection = connection_by_id(&store, &store.workspaces[index].connection_id)?.clone();
    let mut workspace = store.workspaces[index].clone();
    workspace.updated_at = super::super::normalize::now_millis();
    workspace.error = None;
    if enabled {
        start_workspace_session(app, state, &store, &connection, &workspace)?;
    } else {
        stop_workspace_session(app, state, id)?;
    }
    workspace.enabled = enabled;
    store.workspaces[index] = workspace.clone();
    save_mount_store(app, &store)?;
    hydrate_workspace_status(state, &mut workspace);
    Ok(workspace)
}

pub fn refresh_workspace(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    path: Option<String>,
) -> AppResult<MountWorkspace> {
    let rc_addr = {
        let sessions = state
            .mount_sessions
            .lock()
            .map_err(|_| AppError::new("mount_session_lock_failed", "挂载会话状态锁已损坏"))?;
        match sessions.get(id) {
            Some(MountSession::FtpCombine { rc_addr, .. }) => rc_addr.clone(),
            Some(MountSession::NativeSmb { .. }) => None,
            None => return Err(AppError::new("mount_not_running", "挂载工作区未运行")),
        }
    };
    if let Some(rc_addr) = rc_addr {
        ftp_combine::refresh_cache(&rc_addr, path.as_deref())?;
    }
    let mut workspace = load_mount_store(app)?
        .workspaces
        .into_iter()
        .find(|workspace| workspace.id == id)
        .ok_or_else(|| AppError::new("mount_workspace_not_found", "未找到挂载工作区"))?;
    hydrate_workspace_status(state, &mut workspace);
    Ok(workspace)
}

pub fn repair_workspace(app: &AppHandle, state: &AppState, id: &str) -> AppResult<MountWorkspace> {
    let _operation = operation_lock()?;
    let store = load_mount_store(app)?;
    let workspace = store
        .workspaces
        .iter()
        .find(|workspace| workspace.id == id)
        .cloned()
        .ok_or_else(|| AppError::new("mount_workspace_not_found", "未找到挂载工作区"))?;
    let connection = connection_by_id(&store, &workspace.connection_id)?.clone();
    stop_workspace_session(app, state, id)?;
    match workspace.effective_transport {
        Some(EffectiveTransport::NativeSmb) => {
            native_smb::repair_workspace(app, &connection, &workspace)?
        }
        Some(EffectiveTransport::FtpCombine) => ftp_combine::repair_stale_session(app, &workspace)?,
        None => {}
    }
    if workspace.enabled {
        start_workspace_session(app, state, &store, &connection, &workspace)?;
    }
    let mut result = workspace;
    hydrate_workspace_status(state, &mut result);
    Ok(result)
}

fn replace_workspace(app: &AppHandle, workspace: &MountWorkspace) -> AppResult<()> {
    let mut store = load_mount_store(app)?;
    let index = store
        .workspaces
        .iter()
        .position(|item| item.id == workspace.id)
        .ok_or_else(|| AppError::new("mount_workspace_not_found", "未找到挂载工作区"))?;
    store.workspaces[index] = workspace.clone();
    save_mount_store(app, &store)
}
