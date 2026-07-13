/*
 * 核心职责：校验连接、目录唯一性并分配平台本地目标。
 * 能力边界：只处理纯规则与默认路径，不启动网络或挂载进程。
 */

use super::super::*;

pub(super) fn connection_by_id<'a>(
    store: &'a MountStore,
    id: &str,
) -> AppResult<&'a RemoteConnection> {
    store
        .connections
        .iter()
        .find(|connection| connection.id == id)
        .ok_or_else(|| AppError::new("mount_connection_not_found", "未找到远程连接"))
}

pub(super) fn validate_connection_input(input: &RemoteConnectionInput) -> AppResult<()> {
    for (label, value) in [
        ("连接名称", input.name.as_str()),
        ("主机", input.host.as_str()),
        ("用户名", input.username.as_str()),
    ] {
        if value.trim().is_empty() || value.contains(['\r', '\n']) {
            return Err(AppError::new(
                "mount_connection_invalid",
                format!("{}格式无效", label),
            ));
        }
    }
    if matches!(input.ftp_port, Some(0)) || matches!(input.smb_port, Some(0)) {
        return Err(AppError::new("mount_connection_invalid", "端口必须大于 0"));
    }
    Ok(())
}

pub(super) fn ensure_unique_bindings(
    store: &MountStore,
    connection_id: &str,
    workspace_id: &str,
    bindings: &[crate::modules::mounts::dto::RemoteBindingInput],
) -> AppResult<()> {
    let mut paths = Vec::<String>::new();
    for binding in bindings {
        let path = normalize_remote_path(&binding.remote_path);
        if path.is_empty() {
            return Err(AppError::new(
                "mount_remote_path_required",
                "远端目录不能为空",
            ));
        }
        if paths.iter().any(|item| item.eq_ignore_ascii_case(&path)) {
            return Err(AppError::new(
                "mount_binding_duplicate",
                format!("远端目录重复：{}", path),
            ));
        }
        if store.workspaces.iter().any(|workspace| {
            workspace.id != workspace_id
                && workspace.connection_id == connection_id
                && workspace
                    .bindings
                    .iter()
                    .any(|binding| binding.remote_path.eq_ignore_ascii_case(&path))
        }) {
            return Err(AppError::new(
                "mount_binding_exists",
                format!("该连接已挂载远端目录：{}", path),
            ));
        }
        paths.push(path);
    }
    Ok(())
}

pub(super) fn assign_local_targets(
    app: &AppHandle,
    store: &MountStore,
    connection: &RemoteConnection,
    workspace: &mut MountWorkspace,
) -> AppResult<()> {
    #[cfg(windows)]
    assign_windows_targets(app, store, connection, workspace)?;
    #[cfg(not(windows))]
    assign_directory_targets(app, connection, workspace)?;
    Ok(())
}

#[cfg(windows)]
fn assign_windows_targets(
    _app: &AppHandle,
    store: &MountStore,
    _connection: &RemoteConnection,
    workspace: &mut MountWorkspace,
) -> AppResult<()> {
    let mut used = store
        .workspaces
        .iter()
        .filter(|item| item.id != workspace.id)
        .filter_map(|item| item.drive_letter.clone())
        .chain(
            store
                .workspaces
                .iter()
                .filter(|item| item.id != workspace.id)
                .flat_map(|item| {
                    item.bindings
                        .iter()
                        .filter_map(|binding| binding.drive_letter.clone())
                }),
        )
        .collect::<Vec<_>>();
    match workspace.effective_transport {
        Some(EffectiveTransport::NativeSmb) => {
            for binding in &mut workspace.bindings {
                if binding.drive_letter.is_none() {
                    binding.drive_letter = next_drive(&used);
                }
                let letter = binding.drive_letter.clone().ok_or_else(|| {
                    AppError::new("mount_drive_unavailable", "没有可用盘符用于 SMB 共享")
                })?;
                used.push(letter);
            }
            workspace.drive_letter = None;
        }
        Some(EffectiveTransport::FtpCombine) => {
            if workspace.drive_letter.is_none() {
                workspace.drive_letter = next_drive(&used);
            }
            if workspace.drive_letter.is_none() {
                return Err(AppError::new(
                    "mount_drive_unavailable",
                    "没有可用盘符用于 FTP 聚合挂载",
                ));
            }
            workspace
                .bindings
                .iter_mut()
                .for_each(|binding| binding.drive_letter = None);
        }
        None => {}
    }
    Ok(())
}

#[cfg(windows)]
fn next_drive(used: &[String]) -> Option<String> {
    super::super::storage::select_default_drive_letter(used, |letter| {
        PathBuf::from(format!("{}:\\", letter)).exists()
    })
}

#[cfg(not(windows))]
fn assign_directory_targets(
    app: &AppHandle,
    connection: &RemoteConnection,
    workspace: &mut MountWorkspace,
) -> AppResult<()> {
    let (root, _) = super::super::storage::default_mount_root(app)?;
    let workspace_root = root.join(super::super::storage::default_mount_dir_name(&format!(
        "{}-{}",
        connection.name, workspace.name
    )));
    match workspace.effective_transport {
        Some(EffectiveTransport::NativeSmb) => {
            for binding in &mut workspace.bindings {
                if binding.mount_point.is_none() {
                    binding.mount_point = Some(
                        workspace_root
                            .join(super::super::storage::default_mount_dir_name(&binding.name))
                            .to_string_lossy()
                            .to_string(),
                    );
                }
            }
            workspace.mount_point = Some(workspace_root.to_string_lossy().to_string());
        }
        Some(EffectiveTransport::FtpCombine) => {
            workspace
                .mount_point
                .get_or_insert_with(|| workspace_root.to_string_lossy().to_string());
            workspace
                .bindings
                .iter_mut()
                .for_each(|binding| binding.mount_point = None);
        }
        None => {}
    }
    Ok(())
}

pub(super) fn normalize_remote_path(value: &str) -> String {
    value.trim().trim_matches(['/', '\\']).replace('\\', "/")
}

pub(super) fn normalize_tls_mode(value: Option<String>) -> Option<String> {
    match value.as_deref().map(str::trim) {
        Some("explicit") => Some("explicit".to_string()),
        Some("implicit") => Some("implicit".to_string()),
        _ => None,
    }
}

pub(super) fn trim_option(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}
