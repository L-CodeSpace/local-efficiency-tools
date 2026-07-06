/*
 * 核心职责：后台任务模块入口。
 * 业务痛点：任务状态 DTO、业务状态机与 IPC 查询入口需要按领域聚合。
 * 能力边界：只声明任务模块分层。
 */

#[path = "jobs.controller.rs"]
pub mod controller;
#[path = "jobs.dto.rs"]
pub mod dto;
#[path = "jobs.service.rs"]
pub mod service;
