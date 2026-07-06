/*
 * 核心职责：提供 helper 校验和平台工具。
 * 业务痛点：SID、路径白名单和错误映射需要统一。
 * 能力边界：只提供纯工具函数和错误转换。
 */

use super::*;

pub(super) fn current_user_sid() -> AppResult<String> {
    if let Ok(output) = hidden_command("whoami")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(sid) = extract_sid(&stdout) {
                return Ok(sid);
            }
        }
    }

    let output = hidden_command("powershell.exe")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value")
        .output()
        .map_err(|error| {
            AppError::new("hosts_helper_sid_failed", "读取当前 Windows 用户 SID 失败")
                .with_detail(error.to_string())
        })?;
    if !output.status.success() {
        return Err(
            AppError::new("hosts_helper_sid_failed", "读取当前 Windows 用户 SID 失败")
                .with_detail(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    extract_sid(&stdout).ok_or_else(|| {
        AppError::new("hosts_helper_sid_failed", "无法解析当前 Windows 用户 SID")
            .with_detail(stdout.trim().to_string())
    })
}

pub(super) fn extract_sid(value: &str) -> Option<String> {
    value
        .split(|ch: char| ch == '"' || ch == ',' || ch.is_whitespace())
        .map(str::trim)
        .find(|part| part.starts_with("S-1-"))
        .map(ToOwned::to_owned)
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
        platform: "windows".to_string(),
        helper_kind: Some("windowsService".to_string()),
        install_supported: true,
        message,
    }
}

pub(super) fn service_error_code(error: &windows_service::Error) -> Option<u32> {
    match error {
        windows_service::Error::Winapi(error) => error.raw_os_error().map(|code| code as u32),
        _ => None,
    }
}

pub(super) fn service_error_detail(error: &windows_service::Error) -> String {
    match error {
        windows_service::Error::Winapi(error) => error.to_string(),
        _ => error.to_string(),
    }
}

pub(super) fn service_app_error(
    code: &'static str,
    message: &'static str,
) -> impl FnOnce(windows_service::Error) -> AppError {
    move |error| AppError::new(code, message).with_detail(service_error_detail(&error))
}

pub(super) fn is_allowed_hosts_path(path: &Path) -> bool {
    same_path_string(&path.to_string_lossy(), HOSTS_PATH_WINDOWS)
}

pub(super) fn same_path_string(left: &str, right: &str) -> bool {
    normalize_path_string(left) == normalize_path_string(right)
}

pub(super) fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub(super) fn normalize_path_string(value: &str) -> String {
    value
        .trim_matches('"')
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

pub(super) fn current_exe_string() -> AppResult<String> {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|error| {
            AppError::new("hosts_helper_exe_unavailable", "无法读取当前应用路径")
                .with_detail(error.to_string())
        })
}

pub(super) fn hidden_command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

pub(super) fn powershell_quote_path(path: &Path) -> String {
    powershell_quote_string(&path.to_string_lossy())
}

pub(super) fn powershell_quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub(super) fn has_arg(args: &[OsString], arg: &str) -> bool {
    args.iter().any(|value| value == OsStr::new(arg))
}

pub(super) fn arg_value(args: &[OsString], name: &str) -> Option<PathBuf> {
    args.windows(2).find_map(|pair| {
        if pair[0] == OsStr::new(name) {
            Some(PathBuf::from(&pair[1]))
        } else {
            None
        }
    })
}

pub(super) fn json_io_error(error: serde_json::Error) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, error)
}
