/*
 * 核心职责：实现 macOS rclone runtime 挂载目录和可见入口链接。
 * 业务痛点：固定 macFUSE 挂载点易残留 Resource busy，必须隔离真实挂载目录。
 * 能力边界：只处理 macOS 目录、symlink、mount 表和卸载清理。
 */

use super::EffectiveMountTarget;
use crate::modules::mounts::service::{normalize::hidden_command, storage::app_rclone_dir};
use crate::{
    modules::mounts::dto::MountProfile,
    observability,
    shared::error::{AppError, AppResult},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};
use tauri::AppHandle;
use uuid::Uuid;

pub(super) fn prepare_effective_mount_target(
    app: &AppHandle,
    profile: &MountProfile,
    display_target: &Path,
    network_mode: bool,
) -> AppResult<EffectiveMountTarget> {
    cleanup_mount_target(app, display_target, network_mode, &profile.name);
    cleanup_old_runtime_mounts(app, &profile.id, &profile.name)?;
    prepare_display_target(app, display_target)?;
    let actual = runtime_mount_dir(app, &profile.id)?;
    fs::create_dir_all(&actual)?;
    Ok(EffectiveMountTarget {
        actual,
        display: Some(display_target.to_path_buf()),
    })
}

fn runtime_mount_dir(app: &AppHandle, profile_id: &str) -> AppResult<PathBuf> {
    Ok(app_rclone_dir(app)?
        .join("runtime-mounts")
        .join(profile_id)
        .join(Uuid::new_v4().simple().to_string()))
}

fn cleanup_old_runtime_mounts(
    app: &AppHandle,
    profile_id: &str,
    profile_name: &str,
) -> AppResult<()> {
    let base = app_rclone_dir(app)?.join("runtime-mounts").join(profile_id);
    if !base.exists() {
        fs::create_dir_all(&base)?;
        return Ok(());
    }
    for entry in fs::read_dir(&base)? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        cleanup_mount_target(app, &path, false, profile_name);
        if !mount_table_contains(&path) {
            let _ = fs::remove_dir_all(&path);
        }
    }
    Ok(())
}

fn prepare_display_target(app: &AppHandle, display_target: &Path) -> AppResult<()> {
    if let Some(parent) = display_target.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::symlink_metadata(display_target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            replace_known_symlink(app, display_target)
        }
        Ok(metadata) if metadata.is_dir() => prepare_existing_display_dir(app, display_target),
        Ok(_) => Err(target_exists_error(
            display_target,
            "macOS 挂载入口已存在且不是目录或本应用链接，请更换挂载路径。",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(
            AppError::new("mount_target_check_failed", "检查 macOS 挂载入口失败")
                .with_detail(format!("{}: {}", display_target.display(), error)),
        ),
    }
}

fn replace_known_symlink(app: &AppHandle, display_target: &Path) -> AppResult<()> {
    let link_target = fs::read_link(display_target).map_err(|error| {
        AppError::new("mount_target_check_failed", "读取 macOS 挂载入口链接失败")
            .with_detail(format!("{}: {}", display_target.display(), error))
    })?;
    if !is_app_runtime_mount(app, &link_target)? {
        return Err(target_exists_error(
            display_target,
            "macOS 挂载入口已存在且不是本应用链接，请更换挂载路径。",
        ));
    }
    fs::remove_file(display_target)?;
    Ok(())
}

fn prepare_existing_display_dir(app: &AppHandle, display_target: &Path) -> AppResult<()> {
    cleanup_mount_target(app, display_target, false, "macOS 可见挂载点");
    if mount_table_contains(display_target) {
        return Err(AppError::new(
            "mount_target_busy",
            "macOS 挂载路径仍被系统占用，请稍后重试或手动卸载后再挂载。",
        )
        .with_detail(display_target.to_string_lossy().to_string()));
    }
    if fs::read_dir(display_target)?.next().is_none() {
        fs::remove_dir(display_target)?;
        Ok(())
    } else {
        Err(target_exists_error(
            display_target,
            "macOS 挂载入口已存在且不是本应用链接，请更换挂载路径。",
        ))
    }
}

fn is_app_runtime_mount(app: &AppHandle, path: &Path) -> AppResult<bool> {
    let base = app_rclone_dir(app)?.join("runtime-mounts");
    Ok(path.starts_with(base))
}

pub(super) fn create_display_symlink(display_target: &Path, actual_target: &Path) -> AppResult<()> {
    std::os::unix::fs::symlink(actual_target, display_target).map_err(|error| {
        AppError::new("mount_symlink_failed", "创建 macOS 挂载入口链接失败").with_detail(format!(
            "{}: {}",
            display_target.display(),
            error
        ))
    })
}

pub(super) fn cleanup_display_target(
    app: &AppHandle,
    display_target: Option<&Path>,
    actual_target: &Path,
) {
    let Some(display_target) = display_target else {
        return;
    };
    match fs::read_link(display_target) {
        Ok(target) if target == actual_target => {
            let _ = fs::remove_file(display_target);
            observability::emit_info(
                app,
                format!("已清理 macOS rclone 挂载入口: {}", display_target.display()),
            );
        }
        _ => {}
    }
}

pub(super) fn cleanup_mount_target(
    app: &AppHandle,
    target: &Path,
    _network_mode: bool,
    profile_name: &str,
) {
    let target_text = target.to_string_lossy().to_string();
    if target_text.trim().is_empty() {
        return;
    }

    terminate_stale_rclone_mounts(&target_text);
    for _ in 0..5 {
        let diskutil_output = hidden_command(Path::new("/usr/sbin/diskutil"))
            .args(["unmount", "force", target_text.as_str()])
            .output();
        let umount_output = hidden_command(Path::new("/sbin/umount"))
            .args(["-f", target_text.as_str()])
            .output();
        if !mount_table_contains(target) {
            observability::emit_info(
                app,
                format!(
                    "已清理 macOS rclone 挂载点: {} ({})",
                    target.display(),
                    profile_name
                ),
            );
            return;
        }
        log_unmount_failure(app, profile_name, &diskutil_output, &umount_output);
        thread::sleep(Duration::from_millis(300));
    }
    wait_for_mount_release(target);
}

fn wait_for_mount_release(target: &Path) {
    for _ in 0..30 {
        if !mount_table_contains(target) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn mount_table_contains(target: &Path) -> bool {
    let output = hidden_command(Path::new("/sbin/mount")).output();
    let Ok(output) = output else {
        return false;
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let target_text = target.to_string_lossy();
    stdout
        .lines()
        .any(|line| line.contains(&format!(" on {} ", target_text)))
}

fn terminate_stale_rclone_mounts(target_text: &str) {
    let output = Command::new("ps").args(["-axo", "pid=,command="]).output();
    let Ok(output) = output else {
        return;
    };
    let current_pid = std::process::id().to_string();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.contains("rclone") || !line.contains(" mount ") || !line.contains(target_text) {
            continue;
        }
        let Some(pid) = line.split_whitespace().next() else {
            continue;
        };
        if pid == current_pid {
            continue;
        }
        let _ = Command::new("kill").args(["-TERM", pid]).status();
        thread::sleep(Duration::from_millis(200));
        let _ = Command::new("kill").args(["-KILL", pid]).status();
    }
}

fn log_unmount_failure(
    app: &AppHandle,
    profile_name: &str,
    diskutil_output: &std::io::Result<std::process::Output>,
    umount_output: &std::io::Result<std::process::Output>,
) {
    let detail = [
        summarize_command_output("diskutil", diskutil_output),
        summarize_command_output("umount", umount_output),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" | ");
    if !detail.is_empty() {
        observability::emit_info(
            app,
            format!(
                "清理 macOS rclone 挂载点仍未释放 {}: {}",
                profile_name, detail
            ),
        );
    }
}

fn summarize_command_output(name: &str, output: &std::io::Result<std::process::Output>) -> String {
    let Ok(output) = output else {
        return format!("{name}: 启动失败");
    };
    if output.status.success() {
        return String::new();
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let text = if stderr.is_empty() { stdout } else { stderr };
    if text.is_empty() {
        format!("{name}: {}", output.status)
    } else {
        format!("{name}: {text}")
    }
}

fn target_exists_error(target: &Path, message: &'static str) -> AppError {
    AppError::new("mount_target_exists", message).with_detail(target.to_string_lossy().to_string())
}
