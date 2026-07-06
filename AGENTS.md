# AGENTS.md

## Tauri SDK 生成

- 本项目的 Tauri command SDK 生成器不在仓库内。
- 需要生成或检查 SDK 时，到当前用户桌面查找脚本：
  - `%USERPROFILE%\Desktop\CommonlyUsed\scripts\tauri-command-sdk-generator\Cargo.toml`
- 推荐通过项目 npm script 执行：
  - `pnpm generate:sdk`
  - `pnpm check:sdk`
- 如需直接调用，使用：

```powershell
cargo run --manifest-path "%USERPROFILE%\Desktop\CommonlyUsed\scripts\tauri-command-sdk-generator\Cargo.toml" -- --registry src-tauri/src/lib.rs --module-strategy registry
```

- 默认 registry 入口是 `src-tauri/src/lib.rs`，generator 会沿 Rust 模块入口解析实际的 `tauri::generate_handler!` 注册位置。
- SDK 类型中允许使用 `module:function` 分类键，但 Tauri 运行时真实 command 名保持 snake_case，不要改后端命令名来适配分类。
