/*
 * 核心职责：创建最小化临时 rclone 探测配置并安全 obscure 密码。
 * 能力边界：临时配置离开作用域即删除，不写入正式连接配置。
 */

use super::super::*;
use super::super::{normalize::hidden_command, storage::app_rclone_dir};
use super::ProbeKind;
use std::io::Write;

pub(super) fn write_probe_config(
    app: &AppHandle,
    rclone: &Path,
    connection: &RemoteConnection,
    kind: ProbeKind,
) -> AppResult<TemporaryProbeConfig> {
    validate_config_value("主机", &connection.host)?;
    validate_config_value("用户名", &connection.username)?;
    if let Some(domain) = connection.domain.as_deref() {
        validate_config_value("域", domain)?;
    }
    let obscured = obscure_password(rclone, connection.password.as_deref().unwrap_or(""))?;
    let mut lines = vec!["[probe]".to_string()];
    match kind {
        ProbeKind::Smb => append_smb_config(&mut lines, connection),
        ProbeKind::Ftp => append_ftp_config(&mut lines, connection),
    }
    if !obscured.is_empty() {
        lines.push(format!("pass = {}", obscured));
    }
    let directory = app_rclone_dir(app)?.join("probe");
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{}.conf", Uuid::new_v4().simple()));
    fs::write(&path, format!("{}\n", lines.join("\n")))?;
    #[cfg(unix)]
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok(TemporaryProbeConfig { path })
}

fn append_smb_config(lines: &mut Vec<String>, connection: &RemoteConnection) {
    lines.extend([
        "type = smb".to_string(),
        format!("host = {}", connection.host),
        format!("port = {}", connection.smb_port),
        format!("user = {}", connection.username),
    ]);
    if let Some(domain) = connection
        .domain
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        lines.push(format!("domain = {}", domain));
    }
}

fn append_ftp_config(lines: &mut Vec<String>, connection: &RemoteConnection) {
    lines.extend([
        "type = ftp".to_string(),
        format!("host = {}", connection.host),
        format!("port = {}", connection.ftp_port),
        format!("user = {}", connection.username),
        "idle_timeout = 30s".to_string(),
        "close_timeout = 30s".to_string(),
        "shut_timeout = 30s".to_string(),
    ]);
    match connection.tls_mode.as_deref() {
        Some("explicit") => lines.push("explicit_tls = true".to_string()),
        Some("implicit") => lines.push("tls = true".to_string()),
        _ => {}
    }
    if connection.no_check_certificate {
        lines.push("no_check_certificate = true".to_string());
    }
}

pub(in crate::modules::mounts::service) fn obscure_password(
    rclone: &Path,
    password: &str,
) -> AppResult<String> {
    if password.is_empty() {
        return Ok(String::new());
    }
    let mut child = hidden_command(rclone)
        .args(["obscure", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| AppError::new("mount_obscure_failed", "无法写入 rclone 密码输入"))?
        .write_all(format!("{}\n", password).as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(
            AppError::new("mount_obscure_failed", "生成 rclone 密码配置失败")
                .with_detail(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn validate_config_value(label: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() || value.contains(['\r', '\n']) {
        return Err(AppError::new(
            "mount_connection_invalid",
            format!("{}格式无效", label),
        ));
    }
    Ok(())
}

pub(super) struct TemporaryProbeConfig {
    pub(super) path: PathBuf,
}

impl Drop for TemporaryProbeConfig {
    fn drop(&mut self) {
        // 临时探测配置清理失败不应覆盖真实探测结果。
        let _ = fs::remove_file(&self.path);
    }
}
