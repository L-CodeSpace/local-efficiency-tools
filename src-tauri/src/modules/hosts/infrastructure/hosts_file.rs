/*
 * 核心职责：封装系统 hosts 文件读写与解析。
 * 业务痛点：hosts 文件路径和行解析规则必须集中维护，避免 Controller 或 Service 直接碰系统文件。
 * 能力边界：只处理 hosts 文件内容，不决策业务变更计划。
 */

use std::{fs, path::PathBuf};

use crate::{
    modules::hosts::dto::HostEntry,
    shared::error::{AppError, AppResult},
};

pub fn hosts_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    } else if cfg!(target_os = "macos") {
        let canonical_path = PathBuf::from("/private/etc/hosts");
        if canonical_path.exists() {
            canonical_path
        } else {
            PathBuf::from("/etc/hosts")
        }
    } else {
        PathBuf::from("/etc/hosts")
    }
}

pub fn read_hosts() -> AppResult<Vec<HostEntry>> {
    let path = hosts_path();
    let content = fs::read_to_string(&path).map_err(|error| {
        AppError::new("hosts_read_failed", "读取 hosts 文件失败").with_detail(format!(
            "{}: {}",
            path.display(),
            error
        ))
    })?;
    Ok(parse_hosts_content(&content))
}

fn parse_hosts_content(content: &str) -> Vec<HostEntry> {
    content.lines().map(parse_line).collect()
}

fn parse_line(line: &str) -> HostEntry {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return HostEntry {
            raw: line.to_string(),
            ip: None,
            hosts: Vec::new(),
            enabled: false,
            is_comment_or_blank: true,
        };
    }
    let (enabled, candidate) = if let Some(rest) = trimmed.strip_prefix('#') {
        (false, rest.trim())
    } else {
        (true, trimmed.split('#').next().unwrap_or(trimmed).trim())
    };
    let mut parts = candidate.split_whitespace();
    let Some(ip) = parts.next() else {
        return comment(line);
    };
    if !ip
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit() || ch == ':')
    {
        return comment(line);
    }
    let hosts = parts.map(ToOwned::to_owned).collect::<Vec<_>>();
    if hosts.is_empty() {
        return comment(line);
    }
    HostEntry {
        raw: line.to_string(),
        ip: Some(ip.to_string()),
        hosts,
        enabled,
        is_comment_or_blank: false,
    }
}

fn comment(line: &str) -> HostEntry {
    HostEntry {
        raw: line.to_string(),
        ip: None,
        hosts: Vec::new(),
        enabled: false,
        is_comment_or_blank: true,
    }
}
