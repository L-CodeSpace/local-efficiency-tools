/*
 * 核心职责：提供 macOS hosts helper 的平台工具函数。
 * 业务痛点：shell/AppleScript/路径/状态构造需要统一，避免命令注入和文案漂移。
 * 能力边界：只提供纯工具和受控系统命令封装。
 */

use super::*;

pub(super) fn current_uid() -> AppResult<u32> {
    let output = Command::new("id").arg("-u").output().map_err(|error| {
        AppError::new("hosts_helper_uid_failed", "读取当前 macOS 用户 UID 失败")
            .with_detail(error.to_string())
    })?;
    if !output.status.success() {
        return Err(
            AppError::new("hosts_helper_uid_failed", "读取当前 macOS 用户 UID 失败")
                .with_detail(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        );
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .map_err(|error| {
            AppError::new("hosts_helper_uid_failed", "解析当前 macOS 用户 UID 失败")
                .with_detail(error.to_string())
        })
}

pub(super) fn status_with_message(
    installed: bool,
    running: bool,
    token_exists: bool,
    needs_repair: bool,
    message: String,
) -> HostsHelperStatus {
    HostsHelperStatus {
        required: true,
        installed,
        running,
        token_exists,
        needs_repair,
        service_name: Some(SERVICE_NAME.to_string()),
        platform: "macos".to_string(),
        helper_kind: Some("macosLaunchDaemon".to_string()),
        install_supported: true,
        message,
    }
}

pub(super) fn current_exe_string() -> AppResult<String> {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| {
            AppError::new("hosts_helper_exe_unavailable", "无法读取当前应用路径")
                .with_detail(error.to_string())
        })
}

pub(super) fn same_path_string(left: &str, right: &str) -> bool {
    normalize_path_string(left) == normalize_path_string(right)
}

fn normalize_path_string(value: &str) -> String {
    value.trim_matches('"').trim_end_matches('/').to_string()
}

pub(super) fn has_arg(args: &[std::ffi::OsString], arg: &str) -> bool {
    args.iter().any(|value| value == std::ffi::OsStr::new(arg))
}

pub(super) fn arg_value(args: &[std::ffi::OsString], name: &str) -> Option<PathBuf> {
    args.windows(2).find_map(|pair| {
        if pair[0] == std::ffi::OsStr::new(name) {
            Some(PathBuf::from(&pair[1]))
        } else {
            None
        }
    })
}

pub(super) fn json_io_error(error: serde_json::Error) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, error)
}

pub(super) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(super) fn shell_quote_path(path: &Path) -> String {
    shell_quote(&path.to_string_lossy())
}

pub(super) fn apple_script_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub(super) fn run_osascript_admin(
    script: &str,
    code: &'static str,
    message: &'static str,
) -> AppResult<()> {
    let apple_script = format!(
        "do shell script {} with administrator privileges",
        apple_script_quote(script)
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(apple_script)
        .output()
        .map_err(|error| AppError::new(code, message).with_detail(error.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = [
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n");
    Err(
        AppError::new(code, message).with_detail(if detail.is_empty() {
            "请确认已在系统授权弹窗中允许本应用管理 hosts helper。".to_string()
        } else {
            detail
        }),
    )
}
