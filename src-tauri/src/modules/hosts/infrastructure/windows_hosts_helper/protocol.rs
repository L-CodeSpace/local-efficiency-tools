/*
 * 核心职责：定义 helper 服务协议和 CLI 入口。
 * 业务痛点：服务模式、安装模式和 pipe 请求必须共享同一协议。
 * 能力边界：只定义常量、请求结构和命令行分流。
 */

use super::*;

pub const SERVICE_NAME: &str = "LocalEfficiencyToolsHostsHelper";
pub(super) const SERVICE_DISPLAY_NAME: &str = "Local Efficiency Tools Hosts Helper";
pub(super) const SERVICE_DESCRIPTION: &str =
    "Allows Local Efficiency Tools to update the Windows hosts file without repeated UAC prompts.";
pub(super) const PIPE_NAME: &str = r"\\.\pipe\local_efficiency_tools_hosts_helper";
pub(super) const CONFIG_VERSION: u32 = 1;
pub(super) const HELPER_SERVICE_ARG: &str = "--windows-hosts-helper-service";
pub(super) const HELPER_INSTALL_ARG: &str = "--windows-hosts-helper-install";
pub(super) const HELPER_UNINSTALL_ARG: &str = "--windows-hosts-helper-uninstall";
pub(super) const HELPER_CONFIG_ARG: &str = "--helper-config";
pub(super) const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;
pub(super) const PIPE_BUFFER_SIZE: u32 = 64 * 1024;
pub(super) const HOSTS_PATH_WINDOWS: &str = r"C:\Windows\System32\drivers\etc\hosts";

pub(super) static SERVICE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HelperConfig {
    pub(super) version: u32,
    pub(super) token: String,
    pub(super) allowed_user_sid: String,
    pub(super) service_exe: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HelperRequest {
    pub(super) token: String,
    pub(super) action: HelperAction,
    pub(super) content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum HelperAction {
    Ping,
    WriteHosts,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HelperResponse {
    pub(super) ok: bool,
    pub(super) message: String,
    pub(super) detail: Option<String>,
}

pub fn run_cli_if_requested() -> bool {
    let args = std::env::args_os().collect::<Vec<_>>();
    if has_arg(&args, HELPER_SERVICE_ARG) {
        let config_path = arg_value(&args, HELPER_CONFIG_ARG).unwrap_or_else(default_config_path);
        let code = match run_service_dispatcher(config_path) {
            Ok(()) => 0,
            Err(_) => 1,
        };
        std::process::exit(code);
    }

    if has_arg(&args, HELPER_INSTALL_ARG) {
        let code = match arg_value(&args, HELPER_CONFIG_ARG) {
            Some(config_path) => install_or_repair_service(&config_path)
                .map(|_| 0)
                .unwrap_or(1),
            None => 1,
        };
        std::process::exit(code);
    }

    if has_arg(&args, HELPER_UNINSTALL_ARG) {
        let code = uninstall_service().map(|_| 0).unwrap_or(1);
        std::process::exit(code);
    }

    false
}
