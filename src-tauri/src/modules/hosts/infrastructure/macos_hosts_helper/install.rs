/*
 * 核心职责：安装、修复和卸载 macOS LaunchDaemon helper。
 * 业务痛点：系统目录写入和 launchctl 需要一次管理员授权，不能散落在业务流程里。
 * 能力边界：只生成受控安装脚本，不接受外部命令片段。
 */

use super::*;

pub(super) fn install_or_repair_daemon(config_path: &Path) -> AppResult<()> {
    let source_exe = std::env::current_exe().map_err(|error| {
        AppError::new("hosts_helper_exe_unavailable", "无法读取当前应用路径")
            .with_detail(error.to_string())
    })?;
    let plist_content = launch_daemon_plist(config_path);
    let plist_tmp = std::env::temp_dir().join(format!(
        "local-efficiency-hosts-helper-{}.plist",
        Uuid::new_v4().simple()
    ));
    fs::write(&plist_tmp, plist_content).map_err(|error| {
        AppError::new(
            "hosts_helper_plist_failed",
            "创建 macOS hosts helper plist 失败",
        )
        .with_detail(error.to_string())
    })?;

    let script = format!(
        "set -e\nmkdir -p /Library/PrivilegedHelperTools\ncp {source} {helper}\nchown root:wheel {helper}\nchmod 755 {helper}\ncp {plist_tmp} {plist}\nchown root:wheel {plist}\nchmod 644 {plist}\nlaunchctl bootout system/{label} >/dev/null 2>&1 || true\nlaunchctl bootstrap system {plist}\nlaunchctl kickstart -k system/{label}\n",
        source = shell_quote_path(&source_exe),
        helper = shell_quote(HELPER_EXE_PATH),
        plist_tmp = shell_quote_path(&plist_tmp),
        plist = shell_quote(PLIST_PATH),
        label = SERVICE_NAME,
    );
    let result = run_osascript_admin(
        &script,
        "hosts_helper_install_failed",
        "安装 macOS hosts helper 失败",
    );
    let _ = fs::remove_file(&plist_tmp);
    result
}

pub(super) fn uninstall_daemon() -> AppResult<()> {
    let script = format!(
        "set -e\nlaunchctl bootout system/{label} >/dev/null 2>&1 || true\nrm -f {plist} {helper} {socket}\n",
        label = SERVICE_NAME,
        plist = shell_quote(PLIST_PATH),
        helper = shell_quote(HELPER_EXE_PATH),
        socket = shell_quote(SOCKET_PATH),
    );
    run_osascript_admin(
        &script,
        "hosts_helper_uninstall_failed",
        "卸载 macOS hosts helper 失败",
    )
}

fn launch_daemon_plist(config_path: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{helper}</string>
    <string>{daemon_arg}</string>
    <string>{config_arg}</string>
    <string>{config}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/local-efficiency-tools-hosts-helper.out.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/local-efficiency-tools-hosts-helper.err.log</string>
</dict>
</plist>
"#,
        label = escape_plist(SERVICE_NAME),
        helper = escape_plist(HELPER_EXE_PATH),
        daemon_arg = escape_plist(HELPER_DAEMON_ARG),
        config_arg = escape_plist(HELPER_CONFIG_ARG),
        config = escape_plist(&config_path.to_string_lossy()),
    )
}

fn escape_plist(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
