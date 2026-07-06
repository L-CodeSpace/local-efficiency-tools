/*
 * 核心职责：运行 macOS root LaunchDaemon 的 socket 服务。
 * 业务痛点：hosts 写入必须长期由受限后台进程完成，避免主应用反复提权。
 * 能力边界：只接受 ping 和固定 hosts 写入请求，不执行任意命令。
 */

use std::os::unix::net::{UnixListener, UnixStream};

use super::*;

pub(super) fn run_daemon(config_path: &Path) -> AppResult<()> {
    let config = load_config(config_path).map_err(|error| {
        AppError::new(
            "hosts_helper_config_failed",
            "读取 macOS hosts helper 配置失败",
        )
        .with_detail(error.to_string())
    })?;
    let socket_path = Path::new(&config.socket_path);
    let _ = fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path).map_err(|error| {
        AppError::new(
            "hosts_helper_socket_failed",
            "创建 macOS hosts helper socket 失败",
        )
        .with_detail(format!("{}: {}", socket_path.display(), error))
    })?;
    restrict_socket(socket_path, config.allowed_uid)?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let response = handle_stream(&mut stream, &config);
                let _ = write_response(&mut stream, response);
            }
            Err(_) => continue,
        }
    }
    Ok(())
}

fn restrict_socket(socket_path: &Path, uid: u32) -> AppResult<()> {
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600)).map_err(|error| {
        AppError::new(
            "hosts_helper_socket_failed",
            "设置 macOS helper socket 权限失败",
        )
        .with_detail(error.to_string())
    })?;
    let output = Command::new("/usr/sbin/chown")
        .arg(uid.to_string())
        .arg(socket_path)
        .output()
        .map_err(|error| {
            AppError::new(
                "hosts_helper_socket_failed",
                "设置 macOS helper socket 所有者失败",
            )
            .with_detail(error.to_string())
        })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "hosts_helper_socket_failed",
            "设置 macOS helper socket 所有者失败",
        )
        .with_detail(String::from_utf8_lossy(&output.stderr).trim().to_string()))
    }
}

fn handle_stream(
    stream: &mut UnixStream,
    config: &MacosHostsHelperConfig,
) -> MacosHostsHelperResponse {
    match read_message(stream) {
        Ok(bytes) => match serde_json::from_slice::<MacosHostsHelperRequest>(&bytes) {
            Ok(request) => handle_request(request, config),
            Err(error) => response(false, "helper 请求格式无效", Some(error.to_string())),
        },
        Err(error) => response(false, "读取 helper 请求失败", Some(error.to_string())),
    }
}

fn handle_request(
    request: MacosHostsHelperRequest,
    config: &MacosHostsHelperConfig,
) -> MacosHostsHelperResponse {
    if request.token != config.token {
        return response(false, "hosts helper token 不匹配", None);
    }
    match request.action {
        MacosHostsHelperAction::Ping => response(true, "ok", None),
        MacosHostsHelperAction::WriteHosts => match request.content {
            Some(content) => match write_hosts_content_as_daemon(&content) {
                Ok(()) => response(true, "hosts 写入成功", None),
                Err(error) => response(false, error.message, error.detail),
            },
            None => response(false, "缺少 hosts 内容", None),
        },
    }
}

fn write_hosts_content_as_daemon(content: &str) -> AppResult<()> {
    let path = hosts_file::hosts_path();
    if path != Path::new("/private/etc/hosts") && path != Path::new("/etc/hosts") {
        return Err(AppError::new(
            "hosts_helper_path_denied",
            "macOS hosts helper 只允许写入系统 hosts 文件",
        ));
    }
    let backup_path = path.with_extension("bak");
    let tmp_path = PathBuf::from(format!("{}.local-efficiency.tmp", path.display()));
    if path.exists() {
        fs::copy(&path, &backup_path).map_err(|error| {
            AppError::new("hosts_backup_failed", "创建 hosts 备份失败").with_detail(format!(
                "{}: {}",
                backup_path.display(),
                error
            ))
        })?;
    }
    fs::write(&tmp_path, content).map_err(|error| {
        AppError::new("hosts_temp_write_failed", "写入 hosts 临时文件失败").with_detail(format!(
            "{}: {}",
            tmp_path.display(),
            error
        ))
    })?;
    fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o644))?;
    let _ = Command::new("/usr/sbin/chown")
        .arg("root:wheel")
        .arg(&tmp_path)
        .status();
    fs::rename(&tmp_path, &path).map_err(|error| {
        let _ = fs::remove_file(&tmp_path);
        AppError::new("hosts_write_failed", "替换 hosts 文件失败").with_detail(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })
}

fn write_response(stream: &mut UnixStream, response: MacosHostsHelperResponse) -> io::Result<()> {
    let bytes = serde_json::to_vec(&response).map_err(json_io_error)?;
    write_message(stream, &bytes)
}

fn response(
    ok: bool,
    message: impl Into<String>,
    detail: Option<String>,
) -> MacosHostsHelperResponse {
    MacosHostsHelperResponse {
        ok,
        message: message.into(),
        detail,
    }
}
