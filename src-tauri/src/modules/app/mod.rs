/*
 * 核心职责：应用级设置模块入口。
 * 业务痛点：应用设置命令需要作为独立 Controller 暴露给 Tauri IPC。
 * 能力边界：只声明应用级设置入口。
 */

#[path = "app.controller.rs"]
pub mod controller;
