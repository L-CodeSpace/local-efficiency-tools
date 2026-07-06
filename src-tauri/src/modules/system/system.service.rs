/*
 * 核心职责：实现系统信息查询服务。
 * 业务痛点：系统概览和硬件摘要需要与 Tauri Controller 解耦，便于单独测试和复用。
 * 能力边界：只读取本机系统信息，不处理前端展示状态。
 */

use tauri::{AppHandle, Manager};

use crate::{
    modules::system::dto::{HardwareInfo, SystemOverview},
    shared::error::{AppError, AppResult},
};

pub fn overview(app: &AppHandle) -> AppResult<SystemOverview> {
    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        AppError::new("app_data_dir_unavailable", "无法读取应用数据目录")
            .with_detail(error.to_string())
    })?;
    let current_dir = std::env::current_dir()?;
    Ok(SystemOverview {
        app_data_dir: app_data_dir.to_string_lossy().to_string(),
        current_dir: current_dir.to_string_lossy().to_string(),
        platform: std::env::consts::OS.to_string(),
        runtime_policy: "external binaries must be pinned and checksum verified".to_string(),
    })
}

pub fn hardware_info() -> HardwareInfo {
    let system = sysinfo::System::new_all();
    let cpu_name = system
        .cpus()
        .first()
        .map(|cpu| cpu.name().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "未知处理器".to_string());
    let motherboard = sysinfo::Motherboard::new()
        .and_then(|board| {
            let vendor = board.vendor_name().unwrap_or_default();
            let name = board.name().unwrap_or_default();
            let value = format!("{vendor} {name}").trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        })
        .unwrap_or_else(|| "未获取到主板信息".to_string());

    HardwareInfo {
        os_name: sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
        os_version: sysinfo::System::os_version().unwrap_or_else(|| "未知版本".to_string()),
        hostname: sysinfo::System::host_name().unwrap_or_else(|| "未知主机".to_string()),
        cpu_name,
        cpu_cores: system.cpus().len(),
        motherboard,
        ram_total: system.total_memory(),
        ram_used: system.used_memory(),
        swap_total: system.total_swap(),
        swap_used: system.used_swap(),
        gpu_info: Vec::new(),
    }
}
