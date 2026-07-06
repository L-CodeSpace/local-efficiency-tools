/*
 * 核心职责：运行 Windows Service 和 named pipe 服务端。
 * 业务痛点：LocalSystem 服务必须只暴露受限 hosts 写入能力。
 * 能力边界：只处理服务生命周期和服务端请求。
 */

use super::*;

pub(super) fn run_service_dispatcher(config_path: PathBuf) -> windows_service::Result<()> {
    let _ = SERVICE_CONFIG_PATH.set(config_path);
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

define_windows_service!(ffi_service_main, service_main);

pub(super) fn service_main(_arguments: Vec<OsString>) {
    let _ = run_service();
}

pub(super) fn run_service() -> windows_service::Result<()> {
    let config_path = SERVICE_CONFIG_PATH
        .get()
        .cloned()
        .unwrap_or_else(default_config_path);
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(_) => return Ok(()),
    };

    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_handler = Arc::clone(&stop);
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                stop_for_handler.store(true, Ordering::SeqCst);
                let _ = shutdown_tx.send(());
                thread::spawn(wake_pipe);
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let stop_for_listener = Arc::clone(&stop);
    let listener = thread::spawn(move || listener_loop(config, stop_for_listener));
    let _ = shutdown_rx.recv();
    stop.store(true, Ordering::SeqCst);
    wake_pipe();
    let _ = listener.join();

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

pub(super) fn listener_loop(config: HelperConfig, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::SeqCst) {
        let Ok(pipe) = create_server_pipe(&config) else {
            thread::sleep(Duration::from_millis(500));
            continue;
        };

        if connect_pipe(pipe.raw()).is_ok() {
            let _ = handle_connection(pipe.raw(), &config);
            unsafe {
                DisconnectNamedPipe(pipe.raw());
            }
        }
    }
}

pub(super) fn handle_connection(handle: HANDLE, config: &HelperConfig) -> io::Result<()> {
    let request = match read_message(handle) {
        Ok(bytes) => serde_json::from_slice::<HelperRequest>(&bytes).map_err(json_io_error),
        Err(error) => Err(error),
    };
    let response = match request {
        Ok(request) => handle_request(request, config),
        Err(error) => HelperResponse {
            ok: false,
            message: "helper 请求格式无效".to_string(),
            detail: Some(error.to_string()),
        },
    };
    let bytes = serde_json::to_vec(&response).map_err(json_io_error)?;
    write_message(handle, &bytes)
}

pub(super) fn handle_request(request: HelperRequest, config: &HelperConfig) -> HelperResponse {
    if request.token != config.token {
        return HelperResponse {
            ok: false,
            message: "helper token 校验失败".to_string(),
            detail: None,
        };
    }

    match request.action {
        HelperAction::Ping => HelperResponse {
            ok: true,
            message: "ok".to_string(),
            detail: None,
        },
        HelperAction::WriteHosts => {
            let Some(content) = request.content else {
                return HelperResponse {
                    ok: false,
                    message: "hosts 内容不能为空".to_string(),
                    detail: None,
                };
            };
            match write_hosts_content_as_service(&content) {
                Ok(()) => HelperResponse {
                    ok: true,
                    message: "hosts 已写入".to_string(),
                    detail: None,
                },
                Err(error) => HelperResponse {
                    ok: false,
                    message: error.message,
                    detail: error.detail,
                },
            }
        }
    }
}

pub(super) fn write_hosts_content_as_service(content: &str) -> AppResult<()> {
    let path = hosts_file::hosts_path();
    if !is_allowed_hosts_path(&path) {
        return Err(AppError::fatal(
            "hosts_helper_path_rejected",
            "helper 拒绝写入非系统 hosts 路径",
        )
        .with_detail(path.to_string_lossy().to_string()));
    }

    let backup_path = path.with_extension("bak");
    let tmp_path = path.with_file_name(format!(
        "hosts.local-efficiency-tools.{}.tmp",
        Uuid::new_v4().simple()
    ));

    fs::write(&tmp_path, content).map_err(|error| {
        AppError::new("hosts_temp_write_failed", "创建 hosts 临时文件失败").with_detail(format!(
            "{}: {}",
            tmp_path.display(),
            error
        ))
    })?;
    if path.exists() {
        fs::copy(&path, &backup_path).map_err(|error| {
            AppError::new("hosts_backup_failed", "备份 hosts 文件失败").with_detail(format!(
                "{} -> {}: {}",
                path.display(),
                backup_path.display(),
                error
            ))
        })?;
    }
    fs::copy(&tmp_path, &path).map_err(|error| {
        AppError::new("hosts_write_failed", "写入 hosts 文件失败").with_detail(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })?;
    let _ = fs::remove_file(&tmp_path);
    Ok(())
}
