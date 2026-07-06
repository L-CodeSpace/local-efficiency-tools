/*
 * 核心职责：查询和调用 macOS hosts helper。
 * 业务痛点：主应用需要知道 LaunchDaemon 是否可用，并优先通过 helper 写 hosts。
 * 能力边界：只处理状态、安装入口和客户端请求。
 */

use super::*;

pub fn helper_status(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    let config_path = helper_config_path(app)?;
    let config = load_config(&config_path).ok();
    let token_exists = config
        .as_ref()
        .map(|config| !config.token.trim().is_empty())
        .unwrap_or(false);
    let installed = Path::new(PLIST_PATH).exists() && Path::new(HELPER_EXE_PATH).exists();
    if !installed {
        return Ok(status_with_message(
            false,
            false,
            token_exists,
            false,
            "未安装 macOS hosts helper。安装后修改 DNS 不会每次输入管理员密码。".to_string(),
        ));
    }

    let current_exe = current_exe_string().unwrap_or_default();
    let config_matches = config
        .as_ref()
        .map(|config| {
            config.version == CONFIG_VERSION
                && config.allowed_uid == current_uid().unwrap_or_default()
                && same_path_string(&config.source_exe, &current_exe)
                && config.socket_path == SOCKET_PATH
        })
        .unwrap_or(false);
    let running = ping_helper(config.as_ref()).unwrap_or(false);
    let launch_loaded = launchctl_prints_service();
    let needs_repair = !token_exists || !config_matches || !launch_loaded;
    let message = if needs_repair {
        "macOS hosts helper 已安装但配置需要修复。修复时会请求一次管理员授权。".to_string()
    } else if running {
        "macOS hosts helper 正在运行，后续 DNS 修改不会重复输入管理员密码。".to_string()
    } else {
        "macOS hosts helper 已安装但未响应。请修复 helper，失败时将回落到系统授权写入。".to_string()
    };
    Ok(status_with_message(
        true,
        running,
        token_exists,
        needs_repair,
        message,
    ))
}

pub fn install_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    let config_path = write_config_for_current_user(app)?;
    install_or_repair_daemon(&config_path)?;
    helper_status(app)
}

pub fn repair_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    install_helper(app)
}

pub fn uninstall_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    let config_path = helper_config_path(app)?;
    uninstall_daemon()?;
    let _ = fs::remove_file(&config_path);
    helper_status(app)
}

pub fn write_hosts(app: &AppHandle, content: &str) -> AppResult<()> {
    let config_path = helper_config_path(app)?;
    let config = load_config(&config_path).map_err(|error| {
        AppError::new(
            "hosts_helper_unavailable",
            "macOS hosts helper 未安装或配置缺失",
        )
        .with_detail(error.to_string())
    })?;
    let request = MacosHostsHelperRequest {
        token: config.token,
        action: MacosHostsHelperAction::WriteHosts,
        content: Some(content.to_string()),
    };
    let response = send_request(&request)?;
    if response.ok {
        Ok(())
    } else {
        Err(
            AppError::new("hosts_helper_write_failed", response.message).with_detail(
                response
                    .detail
                    .unwrap_or_else(|| "helper 未返回详细错误。".to_string()),
            ),
        )
    }
}

fn ping_helper(config: Option<&MacosHostsHelperConfig>) -> AppResult<bool> {
    let Some(config) = config else {
        return Ok(false);
    };
    let response = send_request(&MacosHostsHelperRequest {
        token: config.token.clone(),
        action: MacosHostsHelperAction::Ping,
        content: None,
    })?;
    Ok(response.ok)
}

fn launchctl_prints_service() -> bool {
    Command::new("launchctl")
        .arg("print")
        .arg(format!("system/{SERVICE_NAME}"))
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
