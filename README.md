# 本地效率工具

本仓库现在按“推倒重写”的目标架构组织：前端是 Vite + React 19 + React Router v7 的 CSR SPA，后端是 Tauri 2 + Rust 分层 use-case 架构。旧的全局任务 store、任意文件 API、rclone 单体、FFmpeg latest 下载流程、VS Code 扩展和 prompt 杂项均不再属于主应用边界。

## 架构边界

- `src/api_tauri/`：前端 IPC SDK 边界，页面禁止直接调用 `invoke`。
- `src/pages/[page]/index.tsx`：页面入口，页面私有 hooks/helpers/types/schema 就近放置。
- `src/components/ui/`：shadcn/ui 基础组件，只允许无业务逻辑的纯 UI。
- `src/components/app/`：应用壳、布局、错误边界、路由级骨架。
- `src/components/common/`：多个页面真实复用的业务组件。
- `src/shared/`：跨页面轻量客户端状态、校验和纯工具。
- `src-tauri/src/domain/`：领域模型和稳定 DTO。
- `src-tauri/src/application/`：用例编排、Job 状态机、preview/confirm/execute。
- `src-tauri/src/infrastructure/`：文件系统、hosts 等外部系统适配。
- `src-tauri/src/ipc/`：Tauri command 注册边界。
- `src-tauri/src/bootstrap/`：应用启动、插件和状态注入。

## 安全策略

危险系统副作用统一走后端用例级 API：

- 文件操作：授权根校验 + preview + confirmation token + execute。
- 批量重命名：后端生成 rename plan，前端只展示并确认。
- hosts 管理：当前只生成变更计划，高权限写入器未启用。
- 长任务：Rust 后端是唯一可信源，前端订阅 `job://updated` 事件。

## 开发命令

```bash
pnpm install
pnpm typecheck
pnpm build
pnpm tauri dev
cargo check --manifest-path src-tauri/Cargo.toml
```

## 当前实现状态

已落地新的目录、路由、SDK 边界、任务骨架、安全文件计划、媒体计划、hosts 计划、空 rclone profile 边界和受限 capabilities。媒体执行器、rclone supervisor、固定版本外部二进制管理和 hosts privileged writer 仍应作为后续 P1/P2 子系统继续实现。
# local-efficiency-tools
