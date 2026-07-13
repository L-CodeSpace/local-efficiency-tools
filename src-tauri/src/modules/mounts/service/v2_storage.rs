/*
 * 核心职责：持久化远程连接与挂载工作区。
 * 业务痛点：连接凭据、目录绑定和运行状态不能继续混存在同一个数组中。
 * 能力边界：只处理 mounts-v2.json，不启动外部进程。
 */

use super::storage::app_rclone_dir;
use super::*;
use std::sync::{Mutex, OnceLock};

const STORE_SCHEMA_VERSION: u16 = 2;
static STORE_IO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn load_mount_store(app: &AppHandle) -> AppResult<MountStore> {
    let _guard = store_io_lock()?;
    load_mount_store_unlocked(app)
}

fn load_mount_store_unlocked(app: &AppHandle) -> AppResult<MountStore> {
    let path = mount_store_path(app)?;
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let mut store: MountStore = serde_json::from_str(&content).map_err(|error| {
            AppError::new("mount_store_parse_failed", "解析远程挂载工作区配置失败")
                .with_detail(error.to_string())
        })?;
        store.schema_version = STORE_SCHEMA_VERSION;
        return Ok(store);
    }

    purge_legacy_mount_data(app)?;
    let store = MountStore {
        schema_version: STORE_SCHEMA_VERSION,
        ..MountStore::default()
    };
    save_mount_store_unlocked(app, &store)?;
    Ok(store)
}

fn purge_legacy_mount_data(app: &AppHandle) -> AppResult<()> {
    let root = app_rclone_dir(app)?;
    let legacy_config = root.join("rclone.conf");
    terminate_legacy_processes(&legacy_config);
    for file in [root.join("profiles.json"), legacy_config] {
        remove_file_if_exists(&file)?;
    }
    for directory in [
        root.join("cache"),
        root.join("logs"),
        root.join("runtime-mounts"),
        root.join("mounts"),
    ] {
        remove_dir_if_exists(&directory)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn remove_dir_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(windows)]
fn terminate_legacy_processes(config: &Path) {
    let config = config.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$config='{}';Get-CimInstance Win32_Process -Filter \"name='rclone.exe'\" | Where-Object {{ $_.CommandLine -like '* mount *' -and $_.CommandLine -like ('*'+$config+'*') }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
        config
    );
    let _ = super::normalize::hidden_command(Path::new("powershell"))
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output();
}

#[cfg(target_os = "macos")]
fn terminate_legacy_processes(config: &Path) {
    let Ok(output) = Command::new("ps").args(["-axo", "pid=,command="]).output() else {
        return;
    };
    let config = config.to_string_lossy();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.contains("rclone") || !line.contains(" mount ") || !line.contains(config.as_ref())
        {
            continue;
        }
        if let Some(pid) = line.split_whitespace().next() {
            let _ = Command::new("kill").args(["-TERM", pid]).status();
        }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn terminate_legacy_processes(_config: &Path) {}

pub(super) fn save_mount_store(app: &AppHandle, store: &MountStore) -> AppResult<()> {
    let _guard = store_io_lock()?;
    save_mount_store_unlocked(app, store)
}

fn save_mount_store_unlocked(app: &AppHandle, store: &MountStore) -> AppResult<()> {
    let path = mount_store_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut persisted = store.clone();
    persisted.schema_version = STORE_SCHEMA_VERSION;
    let content = serde_json::to_string_pretty(&persisted).map_err(|error| {
        AppError::new(
            "mount_store_serialize_failed",
            "序列化远程挂载工作区配置失败",
        )
        .with_detail(error.to_string())
    })?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, content)?;
    if path.exists() {
        fs::remove_file(&path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn store_io_lock() -> AppResult<std::sync::MutexGuard<'static, ()>> {
    STORE_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| AppError::new("mount_store_lock_failed", "挂载配置存储锁已损坏"))
}

pub(super) fn mount_store_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("mounts-v2.json"))
}

pub(super) fn v2_rclone_config_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("mounts-v2.conf"))
}

pub(super) fn workspace_cache_dir(app: &AppHandle, id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?.join("workspace-cache").join(id))
}

pub(super) fn workspace_log_path(app: &AppHandle, id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?
        .join("workspace-logs")
        .join(format!("{}.log", id)))
}

#[cfg(target_os = "macos")]
pub(super) fn workspace_runtime_dir(app: &AppHandle, id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?
        .join("workspace-runtime")
        .join(id)
        .join(Uuid::new_v4().simple().to_string()))
}
