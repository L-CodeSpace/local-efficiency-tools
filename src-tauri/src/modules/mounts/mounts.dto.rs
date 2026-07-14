/*
 * 核心职责：定义远程连接、挂载工作区、平台状态和探测结果契约。
 * 业务痛点：连接凭据、远端目录绑定和运行状态必须使用明确且可扩展的数据模型。
 * 能力边界：只承载可序列化数据，不包含文件、网络或进程副作用。
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
    pub default_drive_letters: Option<Vec<String>>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransportPreference {
    #[default]
    Auto,
    Smb,
    Ftp,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowsSmbAuthMode {
    #[default]
    Auto,
    Plain,
    Domain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EffectiveTransport {
    NativeSmb,
    FtpCombine,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountStatus {
    #[default]
    Disabled,
    Stopped,
    Mounted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MountAdvancedOptions {
    pub dir_cache_time: String,
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
            dir_cache_time: "10s".to_string(),
            vfs_cache_max_size: "5G".to_string(),
            vfs_cache_max_age: "24h".to_string(),
            vfs_read_chunk_size: "64M".to_string(),
            buffer_size: "32M".to_string(),
            poll_interval: "0".to_string(),
            links: false,
            network_mode: false,
            connect_timeout: "10s".to_string(),
            io_timeout: "1m".to_string(),
            retries: 3,
            low_level_retries: 10,
            retries_sleep: "2s".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub ftp_port: u16,
    pub smb_port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_mode: Option<String>,
    pub no_check_certificate: bool,
    #[serde(default)]
    pub transport_preference: TransportPreference,
    #[serde(default)]
    pub windows_auth_mode: WindowsSmbAuthMode,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConnectionInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub username: String,
    pub password: Option<String>,
    pub domain: Option<String>,
    pub ftp_port: Option<u16>,
    pub smb_port: Option<u16>,
    pub tls_mode: Option<String>,
    pub no_check_certificate: Option<bool>,
    pub transport_preference: Option<TransportPreference>,
    pub windows_auth_mode: Option<WindowsSmbAuthMode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmbMappingCleanupItem {
    pub local_name: Option<String>,
    pub remote_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SmbHostCleanupResult {
    pub host: String,
    pub removed_count: u32,
    pub disabled_workspace_count: u32,
    pub removed_mappings: Vec<SmbMappingCleanupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBinding {
    pub id: String,
    pub name: String,
    pub remote_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_letter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_point: Option<String>,
    #[serde(default)]
    pub accessible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBindingInput {
    pub name: String,
    pub remote_path: String,
    pub drive_letter: Option<String>,
    pub mount_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountWorkspace {
    pub id: String,
    pub connection_id: String,
    pub name: String,
    pub bindings: Vec<RemoteBinding>,
    pub drive_letter: Option<String>,
    pub mount_point: Option<String>,
    #[serde(default)]
    pub advanced_options: MountAdvancedOptions,
    pub enabled: bool,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_transport: Option<EffectiveTransport>,
    #[serde(default)]
    pub mounted: bool,
    #[serde(default)]
    pub status: MountStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountWorkspaceInput {
    pub id: Option<String>,
    pub connection_id: String,
    pub name: String,
    pub bindings: Vec<RemoteBindingInput>,
    pub drive_letter: Option<String>,
    pub mount_point: Option<String>,
    pub advanced_options: Option<MountAdvancedOptions>,
    pub enabled: Option<bool>,
    pub effective_transport: Option<EffectiveTransport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeShareEntry {
    pub name: String,
    pub path: String,
    pub accessible: bool,
    pub error: Option<String>,
    pub suggested_drive_letter: Option<String>,
    pub suggested_mount_point: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportProbeResult {
    pub available: bool,
    pub authenticated: bool,
    pub message: String,
    pub raw_output: String,
    pub entries: Vec<ProbeShareEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProbeResult {
    pub connection_id: String,
    pub smb: TransportProbeResult,
    pub ftp: TransportProbeResult,
    pub recommended_transport: Option<EffectiveTransport>,
    pub fallback_reason: Option<String>,
    pub probed_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountStore {
    pub schema_version: u16,
    pub connections: Vec<RemoteConnection>,
    pub workspaces: Vec<MountWorkspace>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundSettings {
    pub enabled: bool,
}
