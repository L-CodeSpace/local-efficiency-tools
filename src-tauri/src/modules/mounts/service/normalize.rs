/*
 * 核心职责：提供挂载模块的无副作用归一化与命令工具。
 * 业务痛点：路径名称、时间戳和隐藏进程创建必须保持一致。
 * 能力边界：不读取业务配置，不启动挂载。
 */

use super::*;

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
