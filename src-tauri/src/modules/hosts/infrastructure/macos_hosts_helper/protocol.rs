/*
 * 核心职责：定义 macOS hosts helper 协议和命令行入口。
 * 业务痛点：主应用和 LaunchDaemon 必须共享稳定、收窄的请求格式。
 * 能力边界：只描述协议常量、请求响应和 helper 模式分流。
 */

use super::*;

pub const SERVICE_NAME: &str = "com.user.local-efficiency-tools.hosts-helper";
pub(super) const HELPER_DAEMON_ARG: &str = "--macos-hosts-helper-daemon";
pub(super) const HELPER_CONFIG_ARG: &str = "--helper-config";
pub(super) const CONFIG_VERSION: u32 = 1;
pub(super) const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;
pub(super) const HELPER_EXE_PATH: &str =
    "/Library/PrivilegedHelperTools/com.user.local-efficiency-tools.hosts-helper";
pub(super) const PLIST_PATH: &str =
    "/Library/LaunchDaemons/com.user.local-efficiency-tools.hosts-helper.plist";
pub(super) const SOCKET_PATH: &str = "/var/run/local-efficiency-tools-hosts-helper.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MacosHostsHelperConfig {
    pub(super) version: u32,
    pub(super) token: String,
    pub(super) allowed_uid: u32,
    pub(super) source_exe: String,
    pub(super) socket_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MacosHostsHelperRequest {
    pub(super) token: String,
    pub(super) action: MacosHostsHelperAction,
    pub(super) content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum MacosHostsHelperAction {
    Ping,
    WriteHosts,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MacosHostsHelperResponse {
    pub(super) ok: bool,
    pub(super) message: String,
    pub(super) detail: Option<String>,
}

pub fn run_cli_if_requested() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if has_arg(&args, HELPER_DAEMON_ARG) {
        let code = match arg_value(&args, HELPER_CONFIG_ARG) {
            Some(config_path) => run_daemon(&config_path).map(|_| 0).unwrap_or(1),
            None => 1,
        };
        std::process::exit(code);
    }
    false
}
