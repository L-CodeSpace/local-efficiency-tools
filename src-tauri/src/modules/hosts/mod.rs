/*
 * 核心职责：hosts 管理模块入口。
 * 业务痛点：hosts DTO、写入服务、系统文件访问和 Windows helper 需要归属同一领域。
 * 能力边界：只声明 hosts 管理模块分层。
 */

#[path = "hosts.controller.rs"]
pub mod controller;
#[path = "hosts.dto.rs"]
pub mod dto;
#[path = "hosts.service.rs"]
pub mod service;

pub mod infrastructure;
