/*
 * 核心职责：查询和调用 Windows hosts helper。
 * 业务痛点：主应用需要明确知道 helper 安装、运行和路径状态。
 * 能力边界：只处理状态查询、安装入口和客户端写入入口。
 */

use super::*;

pub fn helper_status(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    let config_path = helper_config_path(app)?;
    let config = load_config(&config_path).ok();
    let token_exists = config
        .as_ref()
        .map(|config| !config.token.trim().is_empty())
        .unwrap_or(false);
    let current_exe = current_exe_string().unwrap_or_default();
    let manager = match ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
    {
        Ok(manager) => manager,
        Err(error) => {
            return Ok(status_with_message(
                false,
                false,
                token_exists,
                false,
                format!(
                    "无法查询 Windows hosts helper 服务：{}",
                    service_error_detail(&error)
                ),
            ));
        }
    };

    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::QUERY_CONFIG,
    ) {
        Ok(service) => service,
        Err(error) if service_error_code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => {
            return Ok(status_with_message(
                false,
                false,
                token_exists,
                false,
                "未安装 Windows hosts helper。安装后修改 DNS 不会每次触发 UAC。".to_string(),
            ));
        }
        Err(error) => {
            return Ok(status_with_message(
                false,
                false,
                token_exists,
                false,
                format!(
                    "无法读取 Windows hosts helper 状态：{}",
                    service_error_detail(&error)
                ),
            ));
        }
    };

    let running = service
        .query_status()
        .map(|status| status.current_state == ServiceState::Running)
        .unwrap_or(false);
    let launch_command = service
        .query_config()
        .map(|config| config.executable_path.to_string_lossy().to_string())
        .unwrap_or_default();
    let launch_matches = contains_case_insensitive(&launch_command, &current_exe)
        && contains_case_insensitive(&launch_command, HELPER_SERVICE_ARG);
    let config_matches = config
        .as_ref()
        .map(|config| same_path_string(&config.service_exe, &current_exe))
        .unwrap_or(false);
    let needs_repair = !token_exists || !launch_matches || !config_matches;

    let message = if needs_repair {
        "Windows hosts helper 已安装但配置需要修复。修复时会触发一次 UAC。".to_string()
    } else if running {
        "Windows hosts helper 正在运行，后续 DNS 修改不会重复弹出 UAC。".to_string()
    } else {
        "Windows hosts helper 已安装但未运行。请修复 helper 后继续使用。".to_string()
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
    run_elevated_helper(HELPER_INSTALL_ARG, &config_path)?;
    helper_status(app)
}

pub fn repair_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    install_helper(app)
}

pub fn uninstall_helper(app: &AppHandle) -> AppResult<HostsHelperStatus> {
    let config_path = helper_config_path(app)?;
    run_elevated_helper(HELPER_UNINSTALL_ARG, &config_path)?;
    let _ = fs::remove_file(&config_path);
    helper_status(app)
}

pub fn write_hosts(app: &AppHandle, content: &str) -> AppResult<()> {
    let config_path = helper_config_path(app)?;
    let config = load_config(&config_path).map_err(|error| {
        AppError::new(
            "hosts_helper_unavailable",
            "Windows hosts helper 未安装或配置缺失",
        )
        .with_detail(error.to_string())
    })?;
    let request = HelperRequest {
        token: config.token,
        action: HelperAction::WriteHosts,
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
