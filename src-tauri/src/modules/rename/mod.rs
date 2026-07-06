/*
 * 核心职责：批量重命名模块入口。
 * 业务痛点：批量重命名预览和执行计划需要从通用文件操作中拆出独立领域。
 * 能力边界：只声明批量重命名模块分层。
 */

#[path = "rename.controller.rs"]
pub mod controller;
#[path = "rename.dto.rs"]
pub mod dto;
#[path = "rename.service.rs"]
pub mod service;
