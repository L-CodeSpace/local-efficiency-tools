/*
 * 核心职责：系统信息模块入口。
 * 业务痛点：系统 DTO、Service 与 Controller 需要按领域聚合。
 * 能力边界：只声明系统信息模块分层。
 */

#[path = "system.controller.rs"]
pub mod controller;
#[path = "system.dto.rs"]
pub mod dto;
#[path = "system.service.rs"]
pub mod service;
