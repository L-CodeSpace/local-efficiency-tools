/*
 * 核心职责：启动、停止和刷新 FTP combine rclone session。
 * 能力边界：不生成配置内容，不扫描残留进程。
 */

use super::super::*;
use super::super::{
    normalize::hidden_command,
    runtime::check_dependencies,
    runtime_download::ensure_rclone,
    v2_storage::{v2_rclone_config_path, workspace_cache_dir, workspace_log_path},
};
use super::{
    config::{build_mount_args, sync_config},
    stale::repair_stale_session,
};
use crate::modules::state::MountSession;

const INITIAL_EXIT_CHECK: Duration = Duration::from_millis(500);
const RC_TIMEOUT: Duration = Duration::from_secs(10);

pub(in crate::modules::mounts::service) fn start_session(
    app: &AppHandle,
    store: &MountStore,
    _connection: &RemoteConnection,
    workspace: &MountWorkspace,
) -> AppResult<MountSession> {
    repair_stale_session(app, workspace)?;
    let dependency = check_dependencies();
    if !dependency.ready {
        return Err(AppError::new(
            "mount_dependency_missing",
            dependency.message,
        ));
    }
    if workspace.bindings.is_empty() {
        return Err(AppError::new(
            "mount_workspace_empty",
            "工作区至少需要一个远端目录",
        ));
    }
    let rclone = ensure_rclone(app)?;
    sync_config(app, &rclone, store)?;
    let config = v2_rclone_config_path(app)?;
    let cache = workspace_cache_dir(app, &workspace.id)?;
    let log = workspace_log_path(app, &workspace.id)?;
    fs::create_dir_all(&cache)?;
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = prepare_target(app, workspace)?;
    let rc_addr = allocate_rc_addr();
    let mut args = build_mount_args(&config, &cache, workspace, &target);
    if let Some(addr) = rc_addr.as_deref() {
        args.extend([
            "--rc".into(),
            "--rc-no-auth".into(),
            "--rc-addr".into(),
            addr.into(),
        ]);
    }
    let stdout = OpenOptions::new().create(true).append(true).open(&log)?;
    let stderr = stdout.try_clone()?;
    let mut child = hidden_command(&rclone)
        .args(&args)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            AppError::new("mount_start_failed", "启动 FTP 聚合挂载失败")
                .with_detail(error.to_string())
        })?;
    thread::sleep(INITIAL_EXIT_CHECK);
    if let Some(status) = child.try_wait()? {
        cleanup_target(&target, workspace.mount_point.as_deref());
        return Err(
            AppError::new("mount_exited", format!("FTP 聚合挂载已退出: {}", status))
                .with_detail(super::super::normalize::read_tail(&log)),
        );
    }
    let display_target = finish_display_target(workspace, &target)?;
    Ok(MountSession::FtpCombine {
        child,
        workspace_id: workspace.id.clone(),
        workspace_name: workspace.name.clone(),
        target,
        display_target,
        rc_addr: rc_addr.map(|addr| format!("http://{}", addr)),
    })
}

pub(in crate::modules::mounts::service) fn stop_session(
    child: &mut std::process::Child,
    target: &Path,
    display_target: Option<&Path>,
) {
    if let Some(display) = display_target {
        if matches!(fs::read_link(display), Ok(link) if link == target) {
            let _ = fs::remove_file(display);
        }
    }
    terminate_child(child);
    cleanup_target(target, display_target.and_then(Path::to_str));
}

pub(in crate::modules::mounts::service) fn refresh_cache(
    rc_addr: &str,
    path: Option<&str>,
) -> AppResult<()> {
    let client = Client::builder()
        .timeout(RC_TIMEOUT)
        .build()
        .map_err(|error| {
            AppError::new("mount_refresh_cache_failed", "创建 rclone RC 客户端失败")
                .with_detail(error.to_string())
        })?;
    let endpoint = format!("{}/vfs/forget", rc_addr.trim_end_matches('/'));
    let mut request = client.post(endpoint);
    if let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) {
        request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(serde_json::json!({ "dir": path.trim_start_matches(['/', '\\']) }).to_string());
    }
    let response = request.send().map_err(|error| {
        AppError::new("mount_refresh_cache_failed", "刷新 FTP 挂载缓存失败")
            .with_detail(error.to_string())
    })?;
    if !response.status().is_success() {
        return Err(
            AppError::new("mount_refresh_cache_failed", "rclone 拒绝刷新挂载缓存")
                .with_detail(response.status().to_string()),
        );
    }
    Ok(())
}

fn prepare_target(app: &AppHandle, workspace: &MountWorkspace) -> AppResult<PathBuf> {
    #[cfg(windows)]
    {
        let _ = app;
        let drive = workspace.drive_letter.as_deref().unwrap_or("").trim();
        if drive.is_empty() {
            return Err(AppError::new(
                "mount_drive_required",
                "FTP 聚合挂载需要盘符",
            ));
        }
        if PathBuf::from(format!("{}\\", drive.trim_end_matches(['\\', '/']))).exists() {
            return Err(AppError::new(
                "mount_drive_occupied",
                format!("盘符 {} 已被占用", drive),
            ));
        }
        Ok(PathBuf::from(drive))
    }
    #[cfg(target_os = "macos")]
    {
        workspace
            .mount_point
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::new("mount_point_required", "FTP 聚合挂载需要本地入口"))?;
        let target = super::super::v2_storage::workspace_runtime_dir(app, &workspace.id)?;
        fs::create_dir_all(&target)?;
        Ok(target)
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let target = workspace
            .mount_point
            .as_deref()
            .map(PathBuf::from)
            .ok_or_else(|| AppError::new("mount_point_required", "FTP 聚合挂载需要本地入口"))?;
        fs::create_dir_all(&target)?;
        Ok(target)
    }
}

fn finish_display_target(workspace: &MountWorkspace, actual: &Path) -> AppResult<Option<PathBuf>> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::symlink;
        let display = PathBuf::from(workspace.mount_point.as_deref().unwrap_or(""));
        if let Some(parent) = display.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::symlink_metadata(&display) {
            Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(&display)?,
            Ok(_) => {
                return Err(AppError::new(
                    "mount_target_exists",
                    "FTP 聚合挂载入口已存在",
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        symlink(actual, &display)?;
        Ok(Some(display))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (workspace, actual);
        Ok(None)
    }
}

fn cleanup_target(actual: &Path, display: Option<&str>) {
    #[cfg(target_os = "macos")]
    {
        if let Some(display) = display {
            let display = Path::new(display);
            if matches!(fs::read_link(display), Ok(link) if link == actual) {
                let _ = fs::remove_file(display);
            }
        }
        let target = actual.to_string_lossy().to_string();
        let _ = Command::new("/usr/sbin/diskutil")
            .args(["unmount", "force", target.as_str()])
            .output();
        let _ = fs::remove_dir_all(actual);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (actual, display);
    }
}

fn allocate_rc_addr() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").ok()?;
    let address = listener.local_addr().ok()?;
    drop(listener);
    Some(address.to_string())
}

#[cfg(unix)]
fn terminate_child(child: &mut std::process::Child) {
    let pid = child.id().to_string();
    let _ = Command::new("kill").args(["-TERM", &pid]).status();
    for _ in 0..20 {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(unix))]
fn terminate_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}
