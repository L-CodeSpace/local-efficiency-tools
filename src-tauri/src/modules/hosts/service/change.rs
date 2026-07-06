/*
 * 核心职责：计算 hosts 文本变更。
 * 业务痛点：增删改启停必须是可测试的纯文本转换。
 * 能力边界：只处理 hosts 内容变换。
 */

use super::*;

pub(super) fn validate_request(request: &HostsChangeRequest) -> AppResult<()> {
    if request.host.trim().is_empty() {
        return Err(AppError::new("hosts_host_empty", "域名不能为空"));
    }
    if matches!(request.action, HostsChangeAction::Add)
        && request.ip.as_deref().unwrap_or_default().trim().is_empty()
    {
        return Err(AppError::new("hosts_ip_empty", "IP 地址不能为空"));
    }
    Ok(())
}

pub(super) fn apply_change(content: &str, request: &HostsChangeRequest) -> String {
    match request.action {
        HostsChangeAction::Add => add_entry(content, request),
        HostsChangeAction::Remove => remove_entry(content, &request.host),
        HostsChangeAction::Toggle => {
            toggle_entry(content, &request.host, request.enabled.unwrap_or(true))
        }
    }
}

pub(super) fn add_entry(content: &str, request: &HostsChangeRequest) -> String {
    let mut lines = content.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let host = request.host.trim();
    lines.retain(|line| !line_contains_host(line, host));
    lines.push(format!(
        "{} {}",
        request.ip.as_deref().unwrap_or("127.0.0.1").trim(),
        host
    ));
    finish_lines(lines)
}

pub(super) fn remove_entry(content: &str, host: &str) -> String {
    finish_lines(
        content
            .lines()
            .filter(|line| !line_contains_host(line, host))
            .map(ToOwned::to_owned)
            .collect(),
    )
}

pub(super) fn toggle_entry(content: &str, host: &str, enable: bool) -> String {
    finish_lines(
        content
            .lines()
            .map(|line| {
                if !line_contains_host(line, host) {
                    return line.to_string();
                }
                let trimmed = line.trim_start();
                if enable {
                    trimmed
                        .strip_prefix('#')
                        .map(str::trim_start)
                        .unwrap_or(trimmed)
                        .to_string()
                } else if trimmed.starts_with('#') {
                    line.to_string()
                } else {
                    format!("# {line}")
                }
            })
            .collect(),
    )
}

pub(super) fn line_contains_host(line: &str, host: &str) -> bool {
    let trimmed = line.trim_start_matches('#').trim();
    let mut parts = trimmed.split_whitespace();
    let Some(ip) = parts.next() else {
        return false;
    };
    if !ip
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit() || ch == ':')
    {
        return false;
    }
    parts.any(|part| part.eq_ignore_ascii_case(host))
}

pub(super) fn finish_lines(lines: Vec<String>) -> String {
    let mut content = lines.join("\n");
    content.push('\n');
    content
}
