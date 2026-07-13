/*
 * 核心职责：集中维护远程挂载运行时、配置、缓存和默认目标路径。
 * 业务痛点：平台路径和盘符分配必须只有一个可信来源。
 * 能力边界：只处理应用数据目录和轻量 JSON 读取，不管理挂载进程。
 */

use super::normalize::sanitize_path_part;
use super::*;

pub(super) fn read_background_settings(app: &AppHandle) -> AppResult<BackgroundSettings> {
    let path = background_settings_path(app)?;
    if !path.exists() {
        return Ok(BackgroundSettings::default());
    }
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|error| {
        AppError::new("mount_settings_parse_failed", "解析后台运行设置失败")
            .with_detail(error.to_string())
    })
}

pub(super) fn app_rclone_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?.join("rclone");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn rclone_runtime_dir(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("runtime"))
}

pub(super) fn rclone_binary_path(app: &AppHandle) -> AppResult<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        "rclone.exe"
    } else {
        "rclone"
    };
    Ok(rclone_runtime_dir(app)?.join(name))
}

pub(super) fn background_settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("background.json"))
}

pub(super) fn mounts_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app_rclone_dir(app)?.join("mounts");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn default_drive_letter(app: &AppHandle) -> Option<String> {
    default_drive_letters(app, 1).into_iter().next()
}

pub(super) fn default_drive_letters(app: &AppHandle, count: usize) -> Vec<String> {
    #[cfg(windows)]
    {
        let used = read_used_drive_letters(app);
        let mut reserved = used;
        let mut result = Vec::new();
        while result.len() < count {
            let Some(letter) = select_default_drive_letter(&reserved, |letter| {
                PathBuf::from(format!("{}:\\", letter)).exists()
            }) else {
                break;
            };
            reserved.push(letter.clone());
            result.push(letter);
        }
        result
    }
    #[cfg(not(windows))]
    {
        let _ = (app, count);
        Vec::new()
    }
}

#[cfg(windows)]
fn read_used_drive_letters(app: &AppHandle) -> Vec<String> {
    let path = match app_rclone_dir(app) {
        Ok(path) => path.join("mounts-v2.json"),
        Err(_) => return Vec::new(),
    };
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(store) = serde_json::from_str::<MountStore>(&content) else {
        return Vec::new();
    };
    store
        .workspaces
        .iter()
        .filter_map(|workspace| workspace.drive_letter.clone())
        .chain(store.workspaces.iter().flat_map(|workspace| {
            workspace
                .bindings
                .iter()
                .filter_map(|binding| binding.drive_letter.clone())
        }))
        .collect()
}

#[cfg(windows)]
pub(super) fn select_default_drive_letter(
    used_letters: &[String],
    is_occupied: impl Fn(char) -> bool,
) -> Option<String> {
    for code in (b'D'..=b'Z').rev() {
        let letter = code as char;
        if used_letters
            .iter()
            .filter_map(|value| value.chars().next())
            .any(|value| value.eq_ignore_ascii_case(&letter))
            || is_occupied(letter)
        {
            continue;
        }
        return Some(format!("{}:", letter));
    }
    None
}

pub(super) fn default_mount_root(app: &AppHandle) -> AppResult<(PathBuf, bool)> {
    match app.path().desktop_dir() {
        Ok(desktop) => Ok((desktop, true)),
        Err(_) => Ok((mounts_dir(app)?, false)),
    }
}

pub(super) fn default_mount_dir_name(name: &str) -> String {
    sanitize_path_part(name)
}
