/*
 * 核心职责：文件管理模块入口。
 * 业务痛点：文件授权、目录读取和文件操作必须形成清晰的模块边界。
 * 能力边界：只声明文件管理模块分层。
 */

#[path = "file_ops.controller.rs"]
pub mod controller;
#[path = "file_ops.dto.rs"]
pub mod dto;
#[path = "file_ops.service.rs"]
pub mod service;
