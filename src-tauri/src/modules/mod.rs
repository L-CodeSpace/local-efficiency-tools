/*
 * 核心职责：按业务领域聚合后端模块。
 * 业务痛点：Controller、Service、DTO 需要围绕同一业务能力就近组织。
 * 能力边界：只声明模块边界，不实现具体业务流程。
 */

pub mod app;
pub(crate) mod controller_support;
pub mod file_ops;
pub mod hosts;
pub mod jobs;
pub mod media;
pub mod mounts;
pub mod rename;
pub mod shutdown;
pub mod state;
pub mod system;
