/*
 * 核心职责：安装、卸载和等待 Windows Service。
 * 业务痛点：服务注册和修复需要集中封装，避免权限流程散落。
 * 能力边界：只处理 SCM 操作。
 */

use super::*;

pub(super) fn install_or_repair_service(config_path: &Path) -> AppResult<()> {
    let info = service_info(config_path)?;
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .map_err(service_app_error(
        "hosts_helper_manager_failed",
        "无法打开 Windows 服务管理器",
    ))?;

    let service_access = ServiceAccess::QUERY_STATUS
        | ServiceAccess::QUERY_CONFIG
        | ServiceAccess::CHANGE_CONFIG
        | ServiceAccess::START
        | ServiceAccess::STOP;
    match manager.open_service(SERVICE_NAME, service_access) {
        Ok(service) => {
            stop_service_if_running(&service)?;
            service.change_config(&info).map_err(service_app_error(
                "hosts_helper_change_failed",
                "更新 hosts helper 服务失败",
            ))?;
            let _ = service.set_description(SERVICE_DESCRIPTION);
            start_service(&service)?;
            wait_for_service_state(&service, ServiceState::Running, Duration::from_secs(8))?;
        }
        Err(error) if service_error_code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => {
            let service = manager
                .create_service(
                    &info,
                    ServiceAccess::QUERY_STATUS
                        | ServiceAccess::START
                        | ServiceAccess::CHANGE_CONFIG,
                )
                .map_err(service_app_error(
                    "hosts_helper_create_failed",
                    "安装 hosts helper 服务失败",
                ))?;
            let _ = service.set_description(SERVICE_DESCRIPTION);
            start_service(&service)?;
            wait_for_service_state(&service, ServiceState::Running, Duration::from_secs(8))?;
        }
        Err(error) => {
            return Err(
                AppError::new("hosts_helper_open_failed", "打开 hosts helper 服务失败")
                    .with_detail(service_error_detail(&error)),
            );
        }
    }

    Ok(())
}

pub(super) fn uninstall_service() -> AppResult<()> {
    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT).map_err(
            service_app_error("hosts_helper_manager_failed", "无法打开 Windows 服务管理器"),
        )?;
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
    ) {
        Ok(service) => service,
        Err(error) if service_error_code(&error) == Some(ERROR_SERVICE_DOES_NOT_EXIST) => {
            return Ok(());
        }
        Err(error) => {
            return Err(
                AppError::new("hosts_helper_open_failed", "打开 hosts helper 服务失败")
                    .with_detail(service_error_detail(&error)),
            );
        }
    };

    stop_service_if_running(&service)?;
    service.delete().map_err(service_app_error(
        "hosts_helper_delete_failed",
        "卸载 hosts helper 服务失败",
    ))?;
    Ok(())
}

pub(super) fn service_info(config_path: &Path) -> AppResult<ServiceInfo> {
    Ok(ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe().map_err(|error| {
            AppError::new("hosts_helper_exe_unavailable", "无法读取当前应用路径")
                .with_detail(error.to_string())
        })?,
        launch_arguments: vec![
            OsString::from(HELPER_SERVICE_ARG),
            OsString::from(HELPER_CONFIG_ARG),
            config_path.as_os_str().to_os_string(),
        ],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    })
}

pub(super) fn start_service(service: &windows_service::service::Service) -> AppResult<()> {
    match service.start::<&OsStr>(&[]) {
        Ok(()) => Ok(()),
        Err(error) if service_error_code(&error) == Some(ERROR_SERVICE_ALREADY_RUNNING) => Ok(()),
        Err(error) => Err(
            AppError::new("hosts_helper_start_failed", "启动 hosts helper 服务失败")
                .with_detail(service_error_detail(&error)),
        ),
    }
}

pub(super) fn stop_service_if_running(
    service: &windows_service::service::Service,
) -> AppResult<()> {
    let status = service.query_status().map_err(service_app_error(
        "hosts_helper_status_failed",
        "查询 hosts helper 服务状态失败",
    ))?;
    if status.current_state == ServiceState::Stopped {
        return Ok(());
    }
    match service.stop() {
        Ok(_) => {}
        Err(error)
            if matches!(
                service_error_code(&error),
                Some(ERROR_SERVICE_NOT_ACTIVE) | Some(ERROR_SERVICE_DOES_NOT_EXIST)
            ) => {}
        Err(error) => {
            return Err(
                AppError::new("hosts_helper_stop_failed", "停止 hosts helper 服务失败")
                    .with_detail(service_error_detail(&error)),
            );
        }
    }
    wait_for_service_state(service, ServiceState::Stopped, Duration::from_secs(8))
}

pub(super) fn wait_for_service_state(
    service: &windows_service::service::Service,
    target: ServiceState,
    timeout: Duration,
) -> AppResult<()> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        let status = service.query_status().map_err(service_app_error(
            "hosts_helper_status_failed",
            "查询 hosts helper 服务状态失败",
        ))?;
        if status.current_state == target {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(AppError::new(
        "hosts_helper_state_timeout",
        "等待 hosts helper 服务状态超时",
    ))
}
