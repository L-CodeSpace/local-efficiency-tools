/*
 * 核心职责：归一化挂载参数与通用工具。
 * 业务痛点：盘符、remote 名称和日志摘要必须保持一致。
 * 能力边界：只提供纯函数和命令工具。
 */

use super::*;

pub(super) fn normalize_input_drive_letter(value: Option<String>) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        normalize_drive_letter(value)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = value;
        None
    }
}

pub(super) fn normalize_tls_mode(
    protocol: &MountProtocol,
    value: Option<String>,
) -> Option<String> {
    if !matches!(protocol, MountProtocol::Ftp) {
        return None;
    }

    let mode = trim_to_option(value)?.to_ascii_lowercase();
    match mode.as_str() {
        "explicit" | "implicit" => Some(mode),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
pub(super) fn normalize_drive_letter(value: Option<String>) -> Option<String> {
    let value = trim_to_option(value)?;
    let mut chars = value.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    if !letter.is_ascii_alphabetic() {
        return Some(value);
    }
    Some(format!("{}:", letter))
}

pub(super) fn is_drive_target(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.len() == 2 && text.as_bytes().get(1) == Some(&b':')
}

pub(super) fn remote_exists(config_path: &Path, remote_name: &str) -> AppResult<bool> {
    if !config_path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(config_path)?;
    let header = format!("[{}]", remote_name);
    Ok(content
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case(&header)))
}

pub(super) fn delete_remote_from_config(config_path: &Path, remote_name: &str) -> AppResult<()> {
    if !config_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(config_path)?;
    let header = format!("[{}]", remote_name);
    let mut skipping = false;
    let mut next_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            skipping = trimmed.eq_ignore_ascii_case(&header);
            if skipping {
                continue;
            }
        }
        if !skipping {
            next_lines.push(line);
        }
    }

    fs::write(config_path, next_lines.join("\n"))?;
    Ok(())
}

pub(super) fn read_remote_option(
    config_path: &Path,
    remote_name: &str,
    option: &str,
) -> AppResult<Option<String>> {
    if !config_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_path)?;
    let header = format!("[{}]", remote_name);
    let mut in_target = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_target = trimmed.eq_ignore_ascii_case(&header);
            continue;
        }
        if in_target {
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim().eq_ignore_ascii_case(option) {
                    return Ok(Some(value.trim().to_string()));
                }
            }
        }
    }
    Ok(None)
}

pub(super) fn remove_remote_option(
    config_path: &Path,
    remote_name: &str,
    option: &str,
) -> AppResult<()> {
    remove_remote_option_if(config_path, remote_name, option, |_| true)
}

pub(super) fn remove_blank_remote_option(
    config_path: &Path,
    remote_name: &str,
    option: &str,
) -> AppResult<()> {
    remove_remote_option_if(config_path, remote_name, option, |value| {
        value.trim().is_empty()
    })
}

pub(super) fn remove_remote_option_if(
    config_path: &Path,
    remote_name: &str,
    option: &str,
    should_remove: impl Fn(&str) -> bool,
) -> AppResult<()> {
    if !config_path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(config_path)?;
    let header = format!("[{}]", remote_name);
    let mut in_target = false;
    let mut changed = false;
    let mut next_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_target = trimmed.eq_ignore_ascii_case(&header);
            next_lines.push(line);
            continue;
        }
        if in_target {
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim().eq_ignore_ascii_case(option) && should_remove(value) {
                    changed = true;
                    continue;
                }
            }
        }
        next_lines.push(line);
    }

    if changed {
        let mut next_content = next_lines.join("\n");
        if content.ends_with('\n') {
            next_content.push('\n');
        }
        fs::write(config_path, next_content)?;
    }
    Ok(())
}

pub(super) fn push_opt(options: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        options.push((key.to_string(), value.to_string()));
    }
}

pub(super) fn trim_to_option(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn blank(value: &Option<String>) -> bool {
    value.as_deref().map(str::trim).unwrap_or("").is_empty()
}

pub(super) fn sanitize_path_part(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>();
    sanitized.truncate(48);
    if sanitized.trim().is_empty() {
        "remote".to_string()
    } else {
        sanitized
    }
}

pub(super) fn read_tail(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|content| {
            let mut lines = content.lines().rev().take(5).collect::<Vec<_>>();
            lines.reverse();
            lines.join(" | ")
        })
        .unwrap_or_default()
}

pub(super) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn hidden_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        command.creation_flags(0x08000000);
    }
    command
}
