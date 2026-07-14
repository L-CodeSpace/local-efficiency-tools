/*
 * 核心职责：管理远程连接、后台设置和双协议探测入口。
 * 能力边界：不创建工作区，不直接启动挂载会话。
 */

use super::super::*;
use super::super::{
    connection_probe::probe_connection,
    v2_storage::{load_mount_store, save_mount_store},
};
use super::{lifecycle::stop_workspace_session, operation_lock, validation::*};

pub fn list_connections(app: &AppHandle) -> AppResult<Vec<RemoteConnection>> {
    Ok(load_mount_store(app)?.connections)
}

pub fn save_connection(
    app: &AppHandle,
    input: RemoteConnectionInput,
) -> AppResult<RemoteConnection> {
    let _operation = operation_lock()?;
    validate_connection_input(&input)?;
    let mut store = load_mount_store(app)?;
    let now = super::super::normalize::now_millis();
    let existing_index = input
        .id
        .as_ref()
        .and_then(|id| store.connections.iter().position(|item| &item.id == id));
    let existing = existing_index.map(|index| store.connections[index].clone());
    let password = match input.password.as_deref() {
        None => existing.as_ref().and_then(|item| item.password.clone()),
        Some("") => None,
        Some(value) => Some(value.to_string()),
    };
    let connection = RemoteConnection {
        id: existing
            .as_ref()
            .map(|item| item.id.clone())
            .unwrap_or_else(|| Uuid::new_v4().simple().to_string()),
        name: input.name.trim().to_string(),
        host: input.host.trim().to_string(),
        username: input.username.trim().to_string(),
        password,
        domain: trim_option(input.domain),
        ftp_port: input.ftp_port.unwrap_or(21),
        smb_port: input.smb_port.unwrap_or(445),
        tls_mode: normalize_tls_mode(input.tls_mode),
        no_check_certificate: input.no_check_certificate.unwrap_or(false),
        transport_preference: input.transport_preference.unwrap_or_default(),
        windows_auth_mode: input.windows_auth_mode.unwrap_or_default(),
        created_at: existing.as_ref().map(|item| item.created_at).unwrap_or(now),
        updated_at: now,
    };
    if let Some(index) = existing_index {
        store.connections[index] = connection.clone();
    } else {
        store.connections.push(connection.clone());
    }
    save_mount_store(app, &store)?;
    Ok(connection)
}

pub fn delete_connection(app: &AppHandle, state: &AppState, id: &str) -> AppResult<()> {
    let _operation = operation_lock()?;
    let mut store = load_mount_store(app)?;
    if !store.connections.iter().any(|item| item.id == id) {
        return Err(AppError::new(
            "mount_connection_not_found",
            "未找到远程连接",
        ));
    }
    let workspace_ids = store
        .workspaces
        .iter()
        .filter(|workspace| workspace.connection_id == id)
        .map(|workspace| workspace.id.clone())
        .collect::<Vec<_>>();
    for workspace_id in &workspace_ids {
        stop_workspace_session(app, state, workspace_id)?;
    }
    store
        .workspaces
        .retain(|workspace| workspace.connection_id != id);
    store.connections.retain(|connection| connection.id != id);
    save_mount_store(app, &store)
}

pub fn probe(app: &AppHandle, connection_id: &str) -> AppResult<ConnectionProbeResult> {
    let store = load_mount_store(app)?;
    let connection = connection_by_id(&store, connection_id)?;
    probe_connection(app, connection, &store)
}

pub fn get_background_settings(app: &AppHandle) -> AppResult<BackgroundSettings> {
    super::super::storage::read_background_settings(app)
}

pub fn set_background_enabled(app: &AppHandle, enabled: bool) -> AppResult<BackgroundSettings> {
    let _operation = operation_lock()?;
    let settings = BackgroundSettings { enabled };
    let path = super::super::storage::background_settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&settings).map_err(|error| {
            AppError::new("mount_settings_serialize_failed", "序列化后台运行设置失败")
                .with_detail(error.to_string())
        })?,
    )?;
    Ok(settings)
}
