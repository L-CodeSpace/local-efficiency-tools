/*
 * 核心职责：读写 helper 配置并触发提权子进程。
 * 业务痛点：per-install token 和用户 SID 必须集中生成和保存。
 * 能力边界：只处理配置路径、配置文件和提权启动。
 */

use super::*;

pub(super) fn write_config_for_current_user(app: &AppHandle) -> AppResult<PathBuf> {
    let path = helper_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = load_config(&path).ok();
    let token = existing
        .as_ref()
        .map(|config| config.token.trim())
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().simple().to_string());
    let config = HelperConfig {
        version: CONFIG_VERSION,
        token,
        allowed_user_sid: current_user_sid()?,
        service_exe: current_exe_string()?,
    };
    let content = serde_json::to_string_pretty(&config).map_err(|error| {
        AppError::new("hosts_helper_config_failed", "生成 hosts helper 配置失败")
            .with_detail(error.to_string())
    })?;
    fs::write(&path, content).map_err(|error| {
        AppError::new("hosts_helper_config_failed", "写入 hosts helper 配置失败")
            .with_detail(format!("{}: {}", path.display(), error))
    })?;
    Ok(path)
}

pub(super) fn load_config(path: &Path) -> io::Result<HelperConfig> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(json_io_error)
}

pub(super) fn helper_config_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|error| {
        AppError::new("app_data_dir_unavailable", "无法读取应用数据目录")
            .with_detail(error.to_string())
    })?;
    Ok(dir.join("hosts-helper").join("windows-hosts-helper.json"))
}

pub(super) fn default_config_path() -> PathBuf {
    std::env::temp_dir().join("local-efficiency-tools-hosts-helper.json")
}

pub(super) fn run_elevated_helper(mode: &str, config_path: &Path) -> AppResult<()> {
    let exe = std::env::current_exe().map_err(|error| {
        AppError::new("hosts_helper_exe_unavailable", "无法读取当前应用路径")
            .with_detail(error.to_string())
    })?;
    let command = format!(
        "$p = Start-Process -FilePath {exe} -ArgumentList @({mode},{config_arg},{config}) -WindowStyle Hidden -Verb RunAs -Wait -PassThru; if ($null -eq $p) {{ exit 1 }}; exit $p.ExitCode",
        exe = powershell_quote_path(&exe),
        mode = powershell_quote_string(mode),
        config_arg = powershell_quote_string(HELPER_CONFIG_ARG),
        config = powershell_quote_path(config_path)
    );
    let output = hidden_command("powershell.exe")
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command)
        .output()
        .map_err(|error| {
            AppError::new(
                "hosts_helper_elevation_start_failed",
                "启动 helper 管理授权失败",
            )
            .with_detail(error.to_string())
        })?;

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
        Err(AppError::new(
            "hosts_helper_elevation_failed",
            "管理员授权执行 hosts helper 管理失败",
        )
        .with_detail(if detail.is_empty() {
            "请确认已在 UAC 弹窗中允许本应用安装或修复 hosts helper。".to_string()
        } else {
            detail
        }))
    }
}
