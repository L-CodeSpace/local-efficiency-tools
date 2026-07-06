/*
 * 核心职责：定义系统信息模块 DTO。
 * 业务痛点：系统概览和硬件摘要需要稳定传递给前端设置与诊断页面。
 * 能力边界：只描述系统信息契约，不执行硬件探测。
 */

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemOverview {
    pub app_data_dir: String,
    pub current_dir: String,
    pub platform: String,
    pub runtime_policy: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vram: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub cpu_name: String,
    pub cpu_cores: usize,
    pub motherboard: String,
    pub ram_total: u64,
    pub ram_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub gpu_info: Vec<GpuInfo>,
}
