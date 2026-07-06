/*
 * 核心职责：hosts 模块专属基础设施入口。
 * 业务痛点：hosts 文件读写和 Windows helper 只服务 hosts 管理领域。
 * 能力边界：只声明 hosts 专属基础设施。
 */

pub mod hosts_file;
#[cfg(target_os = "macos")]
pub mod macos_hosts_helper;
#[cfg(target_os = "windows")]
pub mod windows_hosts_helper;
