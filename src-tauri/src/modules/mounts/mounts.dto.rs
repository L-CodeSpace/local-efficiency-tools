/*
 * 核心职责：定义远程挂载领域数据结构。
 * 业务痛点：前后端需要共享 rclone 运行时、挂载配置和页面上下文契约。
 * 能力边界：只承载可序列化数据模型，不包含业务流程和外部副作用。
 */

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountRuntimeStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub path: String,
    pub expected_version: String,
    pub download_required: bool,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub download_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountDependencyStatus {
    pub supported: bool,
    pub ready: bool,
    pub dependency_name: String,
    pub installed: bool,
    pub install_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountUiContext {
    pub platform: MountPlatform,
    pub default_mount_root: String,
    pub default_mount_example: String,
    pub default_drive_letter: Option<String>,
    pub config_dir: String,
    pub profile_config_path: String,
    pub rclone_config_path: String,
    pub supports_drive_letter: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MountPlatform {
    Windows,
    Macos,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountProfile {
    pub id: String,
    pub name: String,
    pub protocol: MountProtocol,
    pub remote_name: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    pub url: Option<String>,
    pub vendor: Option<String>,
    pub key_file: Option<String>,
    pub remote_path: Option<String>,
    pub mount_point: Option<String>,
    pub drive_letter: Option<String>,
    pub tls_mode: Option<String>,
    pub no_check_certificate: bool,
    pub read_only: bool,
    pub cache_mode: String,
    #[serde(default)]
    pub advanced_options: MountAdvancedOptions,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub mounted: bool,
    #[serde(default)]
    pub status: MountStatus,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountProfileLog {
    pub profile_id: String,
    pub profile_name: String,
    pub path: String,
    pub exists: bool,
    pub size_bytes: u64,
    pub modified_at: Option<u64>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountProtocol {
    Ftp,
    Sftp,
    Webdav,
}

impl MountProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ftp => "ftp",
            Self::Sftp => "sftp",
            Self::Webdav => "webdav",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountStatus {
    #[default]
    Disabled,
    Stopped,
    Mounted,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountProfileInput {
    pub id: Option<String>,
    pub name: String,
    pub protocol: MountProtocol,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub vendor: Option<String>,
    pub key_file: Option<String>,
    pub remote_path: Option<String>,
    pub mount_point: Option<String>,
    pub drive_letter: Option<String>,
    pub tls_mode: Option<String>,
    pub no_check_certificate: Option<bool>,
    pub read_only: Option<bool>,
    pub cache_mode: Option<String>,
    pub advanced_options: Option<MountAdvancedOptions>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MountAdvancedOptions {
    pub vfs_cache_max_size: String,
    pub vfs_cache_max_age: String,
    pub vfs_read_chunk_size: String,
    pub buffer_size: String,
    pub poll_interval: String,
    pub links: bool,
    pub network_mode: bool,
    pub connect_timeout: String,
    pub io_timeout: String,
    pub retries: u16,
    pub low_level_retries: u16,
    pub retries_sleep: String,
}

impl Default for MountAdvancedOptions {
    fn default() -> Self {
        Self {
            vfs_cache_max_size: "5G".to_string(),
            vfs_cache_max_age: "24h".to_string(),
            vfs_read_chunk_size: "64M".to_string(),
            buffer_size: "32M".to_string(),
            poll_interval: "0".to_string(),
            links: true,
            network_mode: cfg!(target_os = "windows"),
            connect_timeout: "5s".to_string(),
            io_timeout: "30s".to_string(),
            retries: 1,
            low_level_retries: 3,
            retries_sleep: "2s".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountTestResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundSettings {
    pub enabled: bool,
}
