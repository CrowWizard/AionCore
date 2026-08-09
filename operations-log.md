# 操作记录

## 2026-08-09 GUI 启动闪退诊断

- 已在 `aioncore-gui/src/main.rs` 添加发布版启动阶段日志和全局 panic hook。
- 日志文件路径：Windows `%APPDATA%\aioncore-gui\logs\startup.log`。
- 已记录配置读取、后端地址校验、GPUI 创建、GUI 初始化、后端连接和工作区创建阶段。
- 已执行 `cargo fmt --all -- --check` 与 `git diff --check`，均通过。
- `cargo check -p aioncore-gui` 未能在当前 Linux 环境完成：缺少系统依赖 fontconfig（`fontconfig.pc`），与本次 Rust 代码改动无关。
