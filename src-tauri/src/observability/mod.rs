use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppLogEvent {
    pub level: String,
    pub message: String,
}

pub fn emit_info(app: &AppHandle, message: impl Into<String>) {
    emit(app, "info", message);
}

pub fn emit_error(app: &AppHandle, message: impl Into<String>) {
    emit(app, "error", message);
}

fn emit(app: &AppHandle, level: &str, message: impl Into<String>) {
    let _ = app.emit(
        "app://log",
        AppLogEvent {
            level: level.to_string(),
            message: message.into(),
        },
    );
}

pub struct DownloadProgress {
    label: String,
    total: Option<u64>,
    last_percent: u64,
    last_bytes_bucket: u64,
}

impl DownloadProgress {
    pub fn new(label: impl Into<String>, total: Option<u64>) -> Self {
        Self {
            label: label.into(),
            total: total.filter(|value| *value > 0),
            last_percent: 0,
            last_bytes_bucket: 0,
        }
    }

    pub fn record(&mut self, app: &AppHandle, downloaded: u64) {
        if let Some(total) = self.total {
            let percent = downloaded
                .saturating_mul(100)
                .checked_div(total)
                .unwrap_or(0)
                .min(100);
            let bucket = percent / 10 * 10;
            if bucket >= 10 && bucket > self.last_percent {
                emit_info(
                    app,
                    format!(
                        "下载 {} 中: {}% ({}/{})",
                        self.label,
                        bucket,
                        format_bytes(downloaded),
                        format_bytes(total)
                    ),
                );
                self.last_percent = bucket;
            }
            return;
        }

        let bucket = downloaded / UNKNOWN_TOTAL_STEP_BYTES;
        if bucket > self.last_bytes_bucket {
            emit_info(
                app,
                format!(
                    "下载 {} 中: 已下载 {}",
                    self.label,
                    format_bytes(downloaded)
                ),
            );
            self.last_bytes_bucket = bucket;
        }
    }

    pub fn finish(&mut self, app: &AppHandle, downloaded: u64) {
        if let Some(total) = self.total {
            if downloaded >= total && self.last_percent < 100 {
                emit_info(
                    app,
                    format!(
                        "下载 {} 中: 100% ({}/{})",
                        self.label,
                        format_bytes(downloaded),
                        format_bytes(total)
                    ),
                );
                self.last_percent = 100;
            }
        } else if downloaded > 0 {
            emit_info(
                app,
                format!("下载 {} 完成: {}", self.label, format_bytes(downloaded)),
            );
        }
    }
}

const UNKNOWN_TOTAL_STEP_BYTES: u64 = 10 * 1024 * 1024;

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let bytes = bytes as f64;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{} B", bytes as u64)
    }
}
