/*
 * 核心职责：读写 macOS hosts helper 配置。
 * 业务痛点：token、允许 UID 和 socket 路径必须稳定保存，供 root daemon 校验。
 * 能力边界：只处理用户态配置文件，不安装系统服务。
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
    let config = MacosHostsHelperConfig {
        version: CONFIG_VERSION,
        token,
        allowed_uid: current_uid()?,
        source_exe: current_exe_string()?,
        socket_path: SOCKET_PATH.to_string(),
    };
    let content = serde_json::to_string_pretty(&config).map_err(|error| {
        AppError::new(
            "hosts_helper_config_failed",
            "生成 macOS hosts helper 配置失败",
        )
        .with_detail(error.to_string())
    })?;
    fs::write(&path, content).map_err(|error| {
        AppError::new(
            "hosts_helper_config_failed",
            "写入 macOS hosts helper 配置失败",
        )
        .with_detail(format!("{}: {}", path.display(), error))
    })?;
    Ok(path)
}

pub(super) fn helper_config_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|error| {
        AppError::new("app_data_dir_unavailable", "无法读取应用数据目录")
            .with_detail(error.to_string())
    })?;
    Ok(dir.join("hosts-helper").join("macos-hosts-helper.json"))
}

pub(super) fn load_config(path: &Path) -> io::Result<MacosHostsHelperConfig> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(json_io_error)
}
