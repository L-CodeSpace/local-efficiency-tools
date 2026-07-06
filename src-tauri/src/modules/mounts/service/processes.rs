/*
 * 核心职责：管理 rclone mount 进程生命周期。
 * 业务痛点：后台进程必须集中启动、停止和回收，避免资源泄漏。
 * 能力边界：只管理本应用启动的挂载进程。
 */

#[cfg(target_os = "macos")]
use super::target_runtime::create_display_symlink;
use super::*;
use super::{
    normalize::{hidden_command, is_drive_target, read_tail},
    rclone_config::{build_mount_args_for_target, validate_remote_password_config},
    runtime::check_dependencies,
    runtime_download::ensure_rclone,
    storage::{
        load_profiles, profile_cache_dir, profile_log_path, rclone_config_path,
        read_background_settings,
    },
    target::mount_target,
    target_runtime::{
        cleanup_display_target, cleanup_mount_target, prepare_effective_mount_target,
    },
};

pub fn restore_enabled_mounts(app: AppHandle, state: AppState) {
    match load_profiles(&app) {
        Ok(profiles) => {
            for profile in profiles.into_iter().filter(|profile| profile.enabled) {
                if let Err(error) = start_mount(&app, &state, &profile) {
                    observability::emit_info(
                        &app,
                        format!("恢复 rclone 挂载失败 {}: {}", profile.name, error),
                    );
                }
            }
        }
        Err(error) => {
            observability::emit_info(&app, format!("读取 rclone 挂载配置失败: {}", error));
        }
    }
}

pub fn should_hide_on_close(app: &AppHandle, state: &AppState) -> bool {
    active_mount_count(state) > 0
        || read_background_settings(app)
            .map(|settings| settings.enabled)
            .unwrap_or(false)
}

pub fn stop_all(app: &AppHandle, state: &AppState) {
    let running = match state.mount_processes.lock() {
        Ok(mut processes) => processes
            .drain()
            .map(|(_, process)| process)
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };

    for process in running {
        stop_mount_process(app, process);
    }
}

pub(super) fn start_mount(
    app: &AppHandle,
    state: &AppState,
    profile: &MountProfile,
) -> AppResult<()> {
    let deps = check_dependencies();
    if !deps.ready {
        return Err(AppError::new("mount_dependency_missing", deps.message));
    }

    let rclone_path = ensure_rclone(app)?;
    let config_path = rclone_config_path(app)?;
    validate_remote_password_config(&rclone_path, &config_path, &profile.remote_name)?;
    let cache_dir = profile_cache_dir(app, &profile.id)?;
    fs::create_dir_all(&cache_dir)?;

    let display_target = mount_target(profile)?;
    let network_mode = cfg!(target_os = "windows")
        && is_drive_target(&display_target)
        && profile.advanced_options.network_mode;
    stop_mount(app, state, &profile.id)?;
    let target_plan = prepare_effective_mount_target(app, profile, &display_target, network_mode)?;

    let log_path = profile_log_path(app, &profile.id)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr = stdout.try_clone()?;
    let args = build_mount_args_for_target(&config_path, &cache_dir, profile, &target_plan.actual)?;

    observability::emit_info(
        app,
        format!(
            "启动 rclone 挂载: {} -> {}",
            profile.name,
            target_plan
                .display
                .as_ref()
                .unwrap_or(&target_plan.actual)
                .display()
        ),
    );
    if target_plan.display.is_some() {
        observability::emit_info(
            app,
            format!(
                "macOS rclone 实际挂载目录: {}",
                target_plan.actual.display()
            ),
        );
    }
    observability::emit_info(app, format!("rclone mount 参数: {}", args.join(" ")));
    let mut child = hidden_command(&rclone_path)
        .args(&args)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            AppError::new("mount_start_failed", "启动 rclone mount 失败")
                .with_detail(error.to_string())
        })?;

    thread::sleep(Duration::from_millis(900));
    if let Some(status) = child.try_wait()? {
        let details = read_tail(&log_path);
        cleanup_mount_target(app, &target_plan.actual, network_mode, &profile.name);
        cleanup_display_target(app, target_plan.display.as_deref(), &target_plan.actual);
        return Err(AppError::new(
            "mount_exited",
            format!(
                "rclone mount 已退出: {}{}",
                status,
                if details.is_empty() {
                    String::new()
                } else {
                    format!("，日志: {}", details)
                }
            ),
        ));
    }

    #[cfg(target_os = "macos")]
    if let Some(display_target) = target_plan.display.as_deref() {
        if let Err(error) = create_display_symlink(display_target, &target_plan.actual) {
            terminate_mount_child(&mut child);
            cleanup_mount_target(app, &target_plan.actual, network_mode, &profile.name);
            return Err(error);
        }
    }

    state
        .mount_processes
        .lock()
        .map_err(|_| AppError::new("mount_process_lock_failed", "挂载进程状态锁已损坏"))?
        .insert(
            profile.id.clone(),
            MountProcess {
                child,
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                target: target_plan.actual,
                display_target: target_plan.display,
                network_mode,
            },
        );
    observability::emit_info(app, format!("rclone 挂载已启动: {}", profile.name));
    Ok(())
}

pub(super) fn stop_mount(app: &AppHandle, state: &AppState, id: &str) -> AppResult<()> {
    let running = state
        .mount_processes
        .lock()
        .map_err(|_| AppError::new("mount_process_lock_failed", "挂载进程状态锁已损坏"))?
        .remove(id);

    if let Some(process) = running {
        stop_mount_process(app, process);
    }
    Ok(())
}

pub(super) fn active_mount_count(state: &AppState) -> usize {
    match state.mount_processes.lock() {
        Ok(mut processes) => {
            prune_exited(&mut processes);
            processes.len()
        }
        Err(_) => 0,
    }
}

pub(super) fn is_mounted(state: &AppState, id: &str) -> bool {
    match state.mount_processes.lock() {
        Ok(mut processes) => {
            prune_exited(&mut processes);
            processes.contains_key(id)
        }
        Err(_) => false,
    }
}

pub(super) fn prune_exited(processes: &mut HashMap<String, MountProcess>) {
    let exited = processes
        .iter_mut()
        .filter_map(|(id, process)| match process.child.try_wait() {
            Ok(Some(_)) | Err(_) => Some(id.clone()),
            Ok(None) => None,
        })
        .collect::<Vec<_>>();
    for id in exited {
        processes.remove(&id);
    }
}

pub(super) fn hydrate_runtime_status(state: &AppState, profile: &mut MountProfile) {
    profile.mounted = is_mounted(state, &profile.id);
    profile.status = if profile.mounted {
        MountStatus::Mounted
    } else if profile.enabled {
        MountStatus::Stopped
    } else {
        MountStatus::Disabled
    };
    profile.error = None;
}

fn stop_mount_process(app: &AppHandle, mut process: MountProcess) {
    observability::emit_info(app, format!("停止 rclone 挂载: {}", process.profile_name));
    cleanup_display_target(app, process.display_target.as_deref(), &process.target);
    terminate_mount_child(&mut process.child);
    cleanup_mount_target(
        app,
        &process.target,
        process.network_mode,
        &process.profile_name,
    );
}

#[cfg(unix)]
fn terminate_mount_child(child: &mut std::process::Child) {
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
fn terminate_mount_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}
