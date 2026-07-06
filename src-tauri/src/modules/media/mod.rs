/*
 * 核心职责：媒体处理模块入口。
 * 业务痛点：媒体计划、任务执行、运行时下载和探测能力需要按领域聚合。
 * 能力边界：只声明媒体处理模块分层。
 */

#[path = "media.controller.rs"]
pub mod controller;
#[path = "media.dto.rs"]
pub mod dto;
#[path = "media.service.rs"]
pub mod service;
