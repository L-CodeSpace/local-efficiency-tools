/*
 * 核心职责：执行 rclone 目录枚举和逐目录访问验证。
 * 能力边界：所有命令均为只读探测，不持久化业务状态。
 */

use super::super::normalize::hidden_command;
use super::super::*;
use super::{config::write_probe_config, targets::apply_suggested_targets, ProbeKind};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_millis(900);

pub(super) fn probe_transport(
    app: &AppHandle,
    rclone: &Path,
    connection: &RemoteConnection,
    kind: ProbeKind,
    store: &MountStore,
) -> AppResult<TransportProbeResult> {
    let port = match kind {
        ProbeKind::Smb => connection.smb_port,
        ProbeKind::Ftp => connection.ftp_port,
    };
    if !tcp_port_open(&connection.host, port) {
        return Ok(TransportProbeResult {
            available: false,
            authenticated: false,
            message: format!("{}:{} 端口不可达。", connection.host, port),
            raw_output: String::new(),
            entries: Vec::new(),
        });
    }

    let temporary = write_probe_config(app, rclone, connection, kind)?;
    let output = hidden_command(rclone)
        .arg("--config")
        .arg(&temporary.path)
        .args([
            "lsf",
            "probe:",
            "--dirs-only",
            "--max-depth",
            "1",
            "--contimeout",
            "10s",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Ok(TransportProbeResult {
            available: true,
            authenticated: false,
            message: if stderr.is_empty() {
                "认证或目录枚举失败。".to_string()
            } else {
                stderr.clone()
            },
            raw_output: stderr,
            entries: Vec::new(),
        });
    }

    let mut entries = Vec::new();
    for path in parse_probe_paths(&stdout) {
        let access = hidden_command(rclone)
            .arg("--config")
            .arg(&temporary.path)
            .arg("lsf")
            .arg(format!("probe:{}", path))
            .args(["--max-depth", "1", "--contimeout", "10s"])
            .output()?;
        let access_error = (!access.status.success()).then(|| command_error_text(&access));
        entries.push(ProbeShareEntry {
            name: path.rsplit('/').next().unwrap_or(&path).to_string(),
            path,
            accessible: access_error.is_none(),
            error: access_error,
            suggested_drive_letter: None,
            suggested_mount_point: None,
        });
    }
    apply_suggested_targets(app, connection, store, &mut entries)?;
    Ok(TransportProbeResult {
        available: true,
        authenticated: true,
        message: if entries.is_empty() {
            "认证成功，但未发现可访问目录。".to_string()
        } else {
            format!("已发现 {} 个目录。", entries.len())
        },
        raw_output: stdout,
        entries,
    })
}

fn parse_probe_paths(output: &str) -> Vec<String> {
    let mut paths = output
        .lines()
        .map(|line| line.trim().trim_end_matches('/').to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| path.to_lowercase());
    paths.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    paths
}

fn tcp_port_open(host: &str, port: u16) -> bool {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    addresses.into_iter().any(|address: SocketAddr| {
        TcpStream::connect_timeout(&address, PROBE_CONNECT_TIMEOUT).is_ok()
    })
}

fn command_error_text(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    }
}

#[cfg(test)]
mod tests {
    use super::parse_probe_paths;

    #[test]
    fn probe_paths_support_unicode_spaces_and_duplicates() {
        let paths = parse_probe_paths("Video/\n剪辑部 素材/\nhome/\nvideo/\n");
        assert_eq!(paths, vec!["home", "Video", "剪辑部 素材"]);
    }
}
