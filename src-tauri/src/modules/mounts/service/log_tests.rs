/*
 * 核心职责：验证 rclone 日志读取工具。
 * 业务痛点：日志尾部裁剪一旦失效会导致 IPC 返回过大或缺少关键错误。
 * 能力边界：只测试纯函数，不启动 rclone 进程。
 */

#[cfg(test)]
mod log_tests {
    use crate::modules::mounts::service::logs::{
        normalize_log_line_limit, tail_log_content, MAX_LOG_MAX_LINES,
    };

    #[test]
    fn tail_log_content_keeps_latest_lines() {
        let content = "line-1\nline-2\nline-3\nline-4";

        assert_eq!(tail_log_content(content, 2), "line-3\nline-4");
    }

    #[test]
    fn normalize_log_line_limit_caps_zero_and_large_values() {
        assert_eq!(normalize_log_line_limit(0), 1);
        assert_eq!(
            normalize_log_line_limit(MAX_LOG_MAX_LINES + 100),
            MAX_LOG_MAX_LINES
        );
    }
}
