/*
 * 核心职责：远程挂载模块入口。
 * 业务痛点：rclone 配置、挂载进程和依赖检测需要按领域聚合。
 * 能力边界：只声明远程挂载模块分层。
 */

#[path = "mounts.controller.rs"]
pub mod controller;
#[path = "mounts.dto.rs"]
pub mod dto;
#[path = "mounts.service.rs"]
pub mod service;
