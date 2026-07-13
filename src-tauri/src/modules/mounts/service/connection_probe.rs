/*
 * 核心职责：编排 SMB/FTP 只读探测，并选择工作区实际传输方式。
 * 能力边界：不创建挂载、不修改正式 rclone 配置。
 */

use super::*;

#[path = "connection_probe/config.rs"]
mod config;
#[path = "connection_probe/executor.rs"]
mod executor;
#[path = "connection_probe/targets.rs"]
mod targets;

pub(in crate::modules::mounts::service) use config::obscure_password;

pub(super) fn probe_connection(
    app: &AppHandle,
    connection: &RemoteConnection,
    store: &MountStore,
) -> AppResult<ConnectionProbeResult> {
    let rclone = super::runtime_download::ensure_rclone(app)?;
    let smb = match connection.transport_preference {
        TransportPreference::Ftp => skipped_probe("已强制使用 FTP。"),
        _ => executor::probe_transport(app, &rclone, connection, ProbeKind::Smb, store)?,
    };
    let should_probe_ftp = !matches!(connection.transport_preference, TransportPreference::Smb)
        && (!smb.authenticated || !has_accessible_entries(&smb));
    let ftp = if should_probe_ftp
        || matches!(connection.transport_preference, TransportPreference::Ftp)
    {
        executor::probe_transport(app, &rclone, connection, ProbeKind::Ftp, store)?
    } else {
        skipped_probe("SMB 已认证成功，无需执行 FTP 回退探测。")
    };

    let recommended_transport = match connection.transport_preference {
        TransportPreference::Smb if smb.authenticated && has_accessible_entries(&smb) => {
            Some(EffectiveTransport::NativeSmb)
        }
        TransportPreference::Ftp if ftp.authenticated && has_accessible_entries(&ftp) => {
            Some(EffectiveTransport::FtpCombine)
        }
        TransportPreference::Auto if smb.authenticated && has_accessible_entries(&smb) => {
            Some(EffectiveTransport::NativeSmb)
        }
        TransportPreference::Auto if ftp.authenticated && has_accessible_entries(&ftp) => {
            Some(EffectiveTransport::FtpCombine)
        }
        _ => None,
    };
    let fallback_reason = matches!(recommended_transport, Some(EffectiveTransport::FtpCombine))
        .then(|| {
            if smb.message.trim().is_empty() {
                "SMB 不可用，已回退到 FTP 聚合挂载。".to_string()
            } else {
                format!("SMB 不可用，已回退到 FTP 聚合挂载：{}", smb.message)
            }
        });

    Ok(ConnectionProbeResult {
        connection_id: connection.id.clone(),
        smb,
        ftp,
        recommended_transport,
        fallback_reason,
        probed_at: super::normalize::now_millis(),
    })
}

#[derive(Clone, Copy)]
enum ProbeKind {
    Smb,
    Ftp,
}

fn skipped_probe(message: &str) -> TransportProbeResult {
    TransportProbeResult {
        available: false,
        authenticated: false,
        message: message.to_string(),
        raw_output: String::new(),
        entries: Vec::new(),
    }
}

fn has_accessible_entries(result: &TransportProbeResult) -> bool {
    result.entries.iter().any(|entry| entry.accessible)
}
