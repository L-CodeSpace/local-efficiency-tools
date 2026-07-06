/*
 * 核心职责：运行 FFmpeg 子进程并回收任务进程注册。
 * 业务痛点：子进程生命周期、取消和进度 stdout 解析必须与参数生成隔离。
 * 能力边界：只负责单个 FFmpeg 进程运行，不构造转码参数。
 */

use super::*;
pub(super) fn run_ffmpeg_command(
    state: &AppState,
    job_id: &str,
    ffmpeg_path: &Path,
    args: &[String],
    temp_output: &Path,
    mut on_progress: impl FnMut(FfmpegProgress),
) -> FfmpegRunOutcome {
    let mut command = hidden_command(ffmpeg_path);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return FfmpegRunOutcome::Failed(error.to_string()),
    };
    let progress_rx = child.stdout.take().map(|stdout| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                if let Some(progress) = parse_ffmpeg_progress_line(&line) {
                    if tx.send(progress).is_err() {
                        break;
                    }
                }
            }
        });
        rx
    });
    let process_id = child.id();

    if let Err(detail) = register_job_process(state, job_id, child) {
        return FfmpegRunOutcome::Failed(detail);
    }

    loop {
        if let Some(rx) = &progress_rx {
            while let Ok(progress) = rx.try_recv() {
                on_progress(progress);
            }
        }

        if job_is_cancelled(state, job_id) {
            kill_registered_process(state, job_id);
            cleanup_temp(temp_output);
            return FfmpegRunOutcome::Cancelled;
        }

        let finished_child = {
            let mut processes = match state.job_processes.lock() {
                Ok(processes) => processes,
                Err(_) => {
                    return FfmpegRunOutcome::Failed("任务进程注册表锁已损坏".to_string());
                }
            };
            let Some(children) = processes.get_mut(job_id) else {
                return FfmpegRunOutcome::Cancelled;
            };
            let Some(index) = children.iter().position(|child| child.id() == process_id) else {
                return FfmpegRunOutcome::Cancelled;
            };
            let status = match children[index].try_wait() {
                Ok(status) => status,
                Err(error) => {
                    let mut child = children.remove(index);
                    let _ = child.kill();
                    let _ = child.wait();
                    if children.is_empty() {
                        processes.remove(job_id);
                    }
                    return FfmpegRunOutcome::Failed(error.to_string());
                }
            };
            if status.is_some() {
                let child = children.remove(index);
                if children.is_empty() {
                    processes.remove(job_id);
                }
                Some(child)
            } else {
                None
            }
        };

        if let Some(child) = finished_child {
            if let Some(rx) = &progress_rx {
                while let Ok(progress) = rx.try_recv() {
                    on_progress(progress);
                }
            }
            let output = match child.wait_with_output() {
                Ok(output) => output,
                Err(error) => return FfmpegRunOutcome::Failed(error.to_string()),
            };
            if output.status.success() {
                return FfmpegRunOutcome::Succeeded;
            }
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return FfmpegRunOutcome::Failed(if stderr.is_empty() {
                format!("FFmpeg 退出失败: {}", output.status)
            } else {
                stderr
            });
        }

        std::thread::sleep(Duration::from_millis(200));
    }
}

pub(super) fn register_job_process(
    state: &AppState,
    job_id: &str,
    mut child: std::process::Child,
) -> Result<(), String> {
    match state.job_processes.lock() {
        Ok(mut processes) => {
            processes.entry(job_id.to_string()).or_default().push(child);
            Ok(())
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            Err("任务进程注册表锁已损坏".to_string())
        }
    }
}
