/*
 * 核心职责：管理 NativeSmbSession 与 FtpCombineSession 生命周期。
 * 能力边界：不修改连接或目录绑定，只维护运行会话和恢复流程。
 */

use super::super::*;
use super::super::{
    ftp_combine, native_smb,
    v2_storage::{load_mount_store, save_mount_store},
};
use super::{operation_lock, validation::connection_by_id};
use crate::modules::state::MountSession;

pub(super) fn start_workspace_session(
    app: &AppHandle,
    state: &AppState,
    store: &MountStore,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    stop_workspace_session(app, state, &workspace.id)?;
    let session = match workspace.effective_transport {
        Some(EffectiveTransport::NativeSmb) => MountSession::NativeSmb {
            workspace_id: workspace.id.clone(),
            workspace_name: workspace.name.clone(),
            mounts: native_smb::mount_workspace(app, connection, workspace)?,
        },
        Some(EffectiveTransport::FtpCombine) => {
            ftp_combine::start_session(app, store, connection, workspace)?
        }
        None => {
            return Err(AppError::new(
                "mount_transport_missing",
                "挂载工作区尚未选择传输方式",
            ))
        }
    };
    state
        .mount_sessions
        .lock()
        .map_err(|_| AppError::new("mount_session_lock_failed", "挂载会话状态锁已损坏"))?
        .insert(workspace.id.clone(), session);
    Ok(())
}

pub(super) fn stop_workspace_session(app: &AppHandle, state: &AppState, id: &str) -> AppResult<()> {
    let session = state
        .mount_sessions
        .lock()
        .map_err(|_| AppError::new("mount_session_lock_failed", "挂载会话状态锁已损坏"))?
        .remove(id);
    if let Some(session) = session {
        stop_session(app, session);
    }
    Ok(())
}

pub(super) fn forget_native_smb_sessions(
    state: &AppState,
    workspace_ids: &[String],
) -> AppResult<u32> {
    let mut sessions = state
        .mount_sessions
        .lock()
        .map_err(|_| AppError::new("mount_session_lock_failed", "挂载会话状态锁已损坏"))?;
    let before = sessions.len();
    sessions.retain(|id, session| {
        !workspace_ids.iter().any(|workspace_id| workspace_id == id)
            || !matches!(session, MountSession::NativeSmb { .. })
    });
    Ok(before.saturating_sub(sessions.len()) as u32)
}

pub(super) fn hydrate_workspace_status(state: &AppState, workspace: &mut MountWorkspace) {
    workspace.mounted = state
        .mount_sessions
        .lock()
        .map(|sessions| sessions.contains_key(&workspace.id))
        .unwrap_or(false);
    workspace.status = if workspace.mounted {
        MountStatus::Mounted
    } else if workspace.enabled {
        MountStatus::Stopped
    } else {
        MountStatus::Disabled
    };
}

pub(super) fn prune_exited_sessions(app: &AppHandle, state: &AppState) {
    let exited = match state.mount_sessions.lock() {
        Ok(mut sessions) => {
            let ids = sessions
                .iter_mut()
                .filter_map(|(id, session)| {
                    let MountSession::FtpCombine { child, .. } = session else {
                        return None;
                    };
                    (!matches!(child.try_wait(), Ok(None))).then(|| id.clone())
                })
                .collect::<Vec<_>>();
            ids.into_iter()
                .filter_map(|id| sessions.remove(&id))
                .collect::<Vec<_>>()
        }
        Err(_) => Vec::new(),
    };
    for session in exited {
        stop_session(app, session);
    }
}

pub fn restore_enabled_workspaces(app: AppHandle, state: AppState) {
    let Ok(store) = load_mount_store(&app) else {
        return;
    };
    for workspace in store
        .workspaces
        .iter()
        .filter(|workspace| workspace.enabled)
    {
        let Ok(connection) = connection_by_id(&store, &workspace.connection_id) else {
            continue;
        };
        if let Err(error) = start_workspace_session(&app, &state, &store, connection, workspace) {
            observability::emit_info(
                &app,
                format!("恢复挂载工作区失败 {}: {}", workspace.name, error),
            );
        }
    }
}

pub fn stop_all_workspaces(app: &AppHandle, state: &AppState) {
    let sessions = state
        .mount_sessions
        .lock()
        .map(|mut sessions| {
            sessions
                .drain()
                .map(|(_, session)| session)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for session in sessions {
        stop_session(app, session);
    }
}

pub fn unmount_all_workspaces(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let _operation = operation_lock()?;
    stop_all_workspaces(app, state);
    let mut store = load_mount_store(app)?;
    let now = super::super::normalize::now_millis();
    for workspace in &mut store.workspaces {
        workspace.enabled = false;
        workspace.updated_at = now;
    }
    save_mount_store(app, &store)
}

fn stop_session(app: &AppHandle, mut session: MountSession) {
    match &mut session {
        MountSession::NativeSmb { mounts, .. } => native_smb::unmount_workspace(app, mounts),
        MountSession::FtpCombine {
            child,
            target,
            display_target,
            ..
        } => ftp_combine::stop_session(child, target, display_target.as_deref()),
    }
}
