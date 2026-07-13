/*
 * 核心职责：聚合 FTP combine 配置、会话和残留清理能力。
 * 能力边界：只导出 supervisor 所需入口，不承载具体实现。
 */

#[path = "ftp_combine/config.rs"]
mod config;
#[path = "ftp_combine/session.rs"]
mod session;
#[path = "ftp_combine/stale.rs"]
mod stale;

pub(super) use session::{refresh_cache, start_session, stop_session};
pub(super) use stale::repair_stale_session;
