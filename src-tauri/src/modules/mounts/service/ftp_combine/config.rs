/*
 * 核心职责：生成 v2 rclone FTP 与 combine 配置及 mount 参数。
 * 能力边界：不启动进程，不处理本地挂载目标。
 */

use super::super::*;
use super::super::{connection_probe::obscure_password, v2_storage::v2_rclone_config_path};

pub(super) fn sync_config(app: &AppHandle, rclone: &Path, store: &MountStore) -> AppResult<()> {
    let mut content = String::new();
    for connection in &store.connections {
        let workspaces = store
            .workspaces
            .iter()
            .filter(|workspace| {
                workspace.connection_id == connection.id
                    && matches!(
                        workspace.effective_transport,
                        Some(EffectiveTransport::FtpCombine)
                    )
            })
            .collect::<Vec<_>>();
        if workspaces.is_empty() {
            continue;
        }
        append_connection_section(&mut content, rclone, connection)?;
        for workspace in workspaces {
            append_combine_section(&mut content, connection, workspace)?;
        }
    }
    let config = v2_rclone_config_path(app)?;
    if let Some(parent) = config.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config, content)?;
    #[cfg(unix)]
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn append_connection_section(
    content: &mut String,
    rclone: &Path,
    connection: &RemoteConnection,
) -> AppResult<()> {
    validate_ini_value("主机", &connection.host)?;
    validate_ini_value("用户名", &connection.username)?;
    content.push_str(&format!(
        "[{}]\ntype = ftp\nhost = {}\nport = {}\nuser = {}\n",
        connection_remote_name(&connection.id),
        connection.host,
        connection.ftp_port,
        connection.username
    ));
    let password = obscure_password(rclone, connection.password.as_deref().unwrap_or(""))?;
    if !password.is_empty() {
        content.push_str(&format!("pass = {}\n", password));
    }
    match connection.tls_mode.as_deref() {
        Some("explicit") => content.push_str("explicit_tls = true\n"),
        Some("implicit") => content.push_str("tls = true\n"),
        _ => {}
    }
    if connection.no_check_certificate {
        content.push_str("no_check_certificate = true\n");
    }
    content.push_str("idle_timeout = 30s\nclose_timeout = 30s\nshut_timeout = 30s\n\n");
    Ok(())
}

fn append_combine_section(
    content: &mut String,
    connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<()> {
    content.push_str(&format!(
        "[{}]\ntype = combine\nupstreams = {}\n\n",
        workspace_remote_name(&workspace.id),
        combine_upstreams(connection, &workspace.bindings)?
    ));
    Ok(())
}

pub(super) fn combine_upstreams(
    connection: &RemoteConnection,
    bindings: &[RemoteBinding],
) -> AppResult<String> {
    let mut aliases = Vec::<String>::new();
    let mut items = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let path = binding.remote_path.trim().trim_start_matches('/');
        if path.is_empty() || path.contains(['\r', '\n', '"']) {
            return Err(AppError::new(
                "mount_remote_path_invalid",
                "FTP 聚合远端路径无效",
            ));
        }
        let base = binding.name.trim().replace('=', "_");
        if base.is_empty() || base.contains(['\r', '\n', '"']) {
            return Err(AppError::new(
                "mount_binding_name_invalid",
                "FTP 聚合目录名称无效",
            ));
        }
        let mut alias = base.clone();
        let mut suffix = 2usize;
        while aliases
            .iter()
            .any(|value| value.eq_ignore_ascii_case(&alias))
        {
            alias = format!("{}-{}", base, suffix);
            suffix += 1;
        }
        aliases.push(alias.clone());
        let upstream = format!(
            "{}={}:{}",
            alias,
            connection_remote_name(&connection.id),
            path
        )
        .replace('\\', "\\\\");
        items.push(format!("\"{}\"", upstream));
    }
    Ok(items.join(" "))
}

pub(super) fn build_mount_args(
    config: &Path,
    cache: &Path,
    workspace: &MountWorkspace,
    target: &Path,
) -> Vec<String> {
    let options = &workspace.advanced_options;
    vec![
        "--config".into(),
        config.to_string_lossy().into(),
        "mount".into(),
        format!("{}:", workspace_remote_name(&workspace.id)),
        target.to_string_lossy().into(),
        "--cache-dir".into(),
        cache.to_string_lossy().into(),
        "--dir-cache-time".into(),
        options.dir_cache_time.clone(),
        "--vfs-cache-mode".into(),
        "full".into(),
        "--vfs-cache-max-size".into(),
        options.vfs_cache_max_size.clone(),
        "--vfs-cache-max-age".into(),
        options.vfs_cache_max_age.clone(),
        "--vfs-read-chunk-size".into(),
        options.vfs_read_chunk_size.clone(),
        "--buffer-size".into(),
        options.buffer_size.clone(),
        "--poll-interval".into(),
        "0".into(),
        "--contimeout".into(),
        options.connect_timeout.clone(),
        "--retries".into(),
        options.retries.to_string(),
        "--low-level-retries".into(),
        options.low_level_retries.to_string(),
        "--retries-sleep".into(),
        options.retries_sleep.clone(),
        "--log-level".into(),
        "INFO".into(),
    ]
}

fn validate_ini_value(label: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() || value.contains(['\r', '\n']) {
        return Err(AppError::new(
            "mount_connection_invalid",
            format!("{}格式无效", label),
        ));
    }
    Ok(())
}

fn connection_remote_name(id: &str) -> String {
    format!("mount_connection_{}", id)
}

pub(super) fn workspace_remote_name(id: &str) -> String {
    format!("mount_workspace_{}", id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_supports_unicode_spaces_and_duplicate_aliases() {
        let connection = RemoteConnection {
            id: "abc".into(),
            name: "NAS".into(),
            host: "127.0.0.1".into(),
            username: "user".into(),
            password: None,
            domain: None,
            ftp_port: 21,
            smb_port: 445,
            tls_mode: None,
            no_check_certificate: false,
            transport_preference: TransportPreference::Auto,
            created_at: 0,
            updated_at: 0,
        };
        let binding = |id: &str, path: &str| RemoteBinding {
            id: id.into(),
            name: "素材 库".into(),
            remote_path: path.into(),
            drive_letter: None,
            mount_point: None,
            accessible: true,
            error: None,
        };
        let result = combine_upstreams(
            &connection,
            &[binding("1", "Video/素材 库"), binding("2", "home/素材 库")],
        )
        .unwrap();
        assert!(result.contains("素材 库=mount_connection_abc:Video/素材 库"));
        assert!(result.contains("素材 库-2=mount_connection_abc:home/素材 库"));
    }

    #[test]
    fn ftp_mount_args_do_not_include_unsupported_timeout() {
        let workspace = MountWorkspace {
            id: "workspace".into(),
            connection_id: "connection".into(),
            name: "NAS".into(),
            bindings: Vec::new(),
            drive_letter: Some("Z:".into()),
            mount_point: None,
            advanced_options: MountAdvancedOptions::default(),
            enabled: false,
            created_at: 0,
            updated_at: 0,
            effective_transport: Some(EffectiveTransport::FtpCombine),
            mounted: false,
            status: MountStatus::Disabled,
            error: None,
        };
        let args = build_mount_args(
            Path::new("config"),
            Path::new("cache"),
            &workspace,
            Path::new("Z:"),
        );
        assert!(!args.iter().any(|argument| argument == "--timeout"));
        assert!(args.iter().any(|argument| argument == "--contimeout"));
    }
}
