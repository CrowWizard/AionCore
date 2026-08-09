# 操作记录

## 2026-08-09 GUI 启动闪退诊断

- 已在 `aioncore-gui/src/main.rs` 添加发布版启动阶段日志和全局 panic hook。
- 日志文件路径：Windows `%APPDATA%\aioncore-gui\logs\startup.log`。
- 已记录配置读取、后端地址校验、GPUI 创建、GUI 初始化、后端连接和工作区创建阶段。
- 已执行 `cargo fmt --all -- --check` 与 `git diff --check`，均通过。
- `cargo check -p aioncore-gui` 未能在当前 Linux 环境完成：缺少系统依赖 fontconfig（`fontconfig.pc`），与本次 Rust 代码改动无关。

## 2026-08-09 GUI Tokio reactor panic 修复

- 根据 Windows 启动日志，确认窗口创建完成后发生 `there is no reactor running` panic。
- 将 GUI 的 HTTP 请求和文件上传显式提交至 `CoreClient` 共享的 Tokio runtime，避免 GPUI executor 直接轮询 Tokio I/O。
- 将团队事件空轮询改为 `smol::Timer`，避免 GPUI executor 使用 Tokio timer。
- panic 日志追加强制捕获的 backtrace，便于后续定位。
- 已使用 `RUST_FONTCONFIG_DLOPEN=1 cargo check -p aioncore-gui` 完成定向编译检查。

## 2026-08-09 AionCore 项目目录切换

- 项目面板点击项目后，会解析其已授权且可用的 workspace 根目录。
- 已加入项目 workspace 切换 action，切换 GUI 全局工作目录。
- 已打开终端会自动以所选项目目录重新启动，运行中的终端命令会被中断。
- 已完成 `RUST_FONTCONFIG_DLOPEN=1 cargo check -p aioncore-gui` 定向编译检查。

## 2026-08-10 AionCode 会话创建与目录绑定

- 新建 Conversation 前拉取启用的 AionCore assistants，并按 assistant 展示创建入口。
- 创建时要求选择 workspace 目录，将其通过 `extra.workspace` 传入 Conversation API。
- 创建请求携带 `assistant.id`，满足后端会话创建契约。
- 默认布局移除依赖可选 Project Explorer 路由的 Project 面板，兼容不含 `/api/projects` 的后端。
- 已完成 `RUST_FONTCONFIG_DLOPEN=1 cargo check -p aioncore-gui` 定向编译检查。
