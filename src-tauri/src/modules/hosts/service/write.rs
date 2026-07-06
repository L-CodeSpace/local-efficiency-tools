/*
 * 核心职责：执行 hosts 写入和提权 fallback。
 * 业务痛点：系统 hosts 写入有权限边界，必须集中处理失败路径。
 * 能力边界：只处理写文件、helper 和一次性提权。
 */

use super::*;

pub(super) fn write_hosts_content(app: &AppHandle, path: &Path, content: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    if path == hosts_file::hosts_path() {
        if windows_hosts_helper::write_hosts(app, content).is_ok() {
            return Ok(());
        }
    }

    #[cfg(target_os = "macos")]
    if path == hosts_file::hosts_path() {
        if macos_hosts_helper::write_hosts(app, content).is_ok() {
            return Ok(());
        }
    }

    match fs::write(path, content) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            elevate_and_write(path, content)
        }
        Err(error) => Err(AppError::new("hosts_write_failed", "写入 hosts 文件失败")
            .with_detail(format!("{}: {}", path.display(), error))),
    }
}

pub(super) fn elevate_and_write(path: &Path, content: &str) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        return elevate_and_write_windows(path, content);
    }

    #[cfg(target_os = "macos")]
    {
        return elevate_and_write_macos(path, content);
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (path, content);
        Err(AppError::new(
            "hosts_permission_denied",
            "写入 hosts 文件需要管理员权限",
        ))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn elevate_and_write_macos(path: &Path, content: &str) -> AppResult<()> {
    let token = Uuid::new_v4().simple().to_string();
    let tmp_path = std::env::temp_dir().join(format!("local_efficiency_hosts_{token}.tmp"));
    let backup_path = path.with_extension("bak");

    fs::write(&tmp_path, content).map_err(|error| {
        AppError::new("hosts_temp_write_failed", "创建 hosts 临时文件失败").with_detail(format!(
            "{}: {}",
            tmp_path.display(),
            error
        ))
    })?;

    let shell_script = format!(
        "if [ -f {path} ]; then cp -p {path} {backup}; fi\ncp {tmp} {path}\nchmod 0644 {path}\nchown root:wheel {path}",
        path = shell_quote_path(path),
        backup = shell_quote_path(&backup_path),
        tmp = shell_quote_path(&tmp_path),
    );
    let apple_script = format!(
        "do shell script {} with administrator privileges",
        apple_script_quote(&shell_script)
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(apple_script)
        .output()
        .map_err(|error| {
            AppError::new("hosts_elevation_start_failed", "启动 macOS 管理员授权失败")
                .with_detail(error.to_string())
        })?;

    let _ = fs::remove_file(&tmp_path);

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Err(
            AppError::new("hosts_elevation_failed", "macOS 管理员授权写入 hosts 失败").with_detail(
                if detail.is_empty() {
                    format!(
                        "请确认已在系统授权弹窗中允许本应用修改 {}。",
                        path.display()
                    )
                } else {
                    detail
                },
            ),
        )
    }
}

#[cfg(target_os = "windows")]
pub(super) fn elevate_and_write_windows(path: &Path, content: &str) -> AppResult<()> {
    let token = Uuid::new_v4().simple().to_string();
    let temp_dir = std::env::temp_dir();
    let tmp_path = temp_dir.join(format!("local_efficiency_hosts_{token}.tmp"));
    let script_path = temp_dir.join(format!("local_efficiency_hosts_{token}.ps1"));
    let backup_path = path.with_extension("bak");

    fs::write(&tmp_path, content).map_err(|error| {
        AppError::new("hosts_temp_write_failed", "创建 hosts 临时文件失败").with_detail(format!(
            "{}: {}",
            tmp_path.display(),
            error
        ))
    })?;

    let script_content = format!(
        "$ErrorActionPreference = 'Stop'\nif (Test-Path -LiteralPath {path}) {{ Copy-Item -LiteralPath {path} -Destination {backup} -Force }}\nCopy-Item -LiteralPath {tmp} -Destination {path} -Force\n",
        path = powershell_quote_path(path),
        backup = powershell_quote_path(&backup_path),
        tmp = powershell_quote_path(&tmp_path),
    );
    fs::write(&script_path, script_content).map_err(|error| {
        AppError::new("hosts_script_write_failed", "创建 hosts 提权脚本失败").with_detail(format!(
            "{}: {}",
            script_path.display(),
            error
        ))
    })?;

    let command = format!(
        "$p = Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile','-ExecutionPolicy','Bypass','-File',{script}) -WindowStyle Hidden -Verb RunAs -Wait -PassThru; if ($null -eq $p) {{ exit 1 }}; exit $p.ExitCode",
        script = powershell_quote_path(&script_path)
    );

    let output = hidden_command("powershell.exe")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command)
        .output()
        .map_err(|error| {
            AppError::new("hosts_elevation_start_failed", "启动管理员授权失败")
                .with_detail(error.to_string())
        })?;

    let _ = fs::remove_file(&tmp_path);
    let _ = fs::remove_file(&script_path);

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = [stdout, stderr]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Err(
            AppError::new("hosts_elevation_failed", "管理员授权写入 hosts 失败").with_detail(
                if detail.is_empty() {
                    "请确认已在 UAC 弹窗中允许本应用修改 hosts 文件。".to_string()
                } else {
                    detail
                },
            ),
        )
    }
}

#[cfg(target_os = "macos")]
pub(super) fn shell_quote_path(path: &Path) -> String {
    shell_quote_string(&path.to_string_lossy())
}

#[cfg(target_os = "macos")]
pub(super) fn shell_quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
pub(super) fn apple_script_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(target_os = "windows")]
pub(super) fn powershell_quote_path(path: &Path) -> String {
    powershell_quote_string(&path.to_string_lossy())
}

#[cfg(target_os = "windows")]
pub(super) fn powershell_quote_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub(super) fn hidden_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}
