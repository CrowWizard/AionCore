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

## 2026-08-10 aionrs 持久会话列表

- 新增受认证保护的 `GET /api/aionrs/sessions`，由 AionCore 读取 `<data-dir>/aionrs-sessions` 并仅返回 aionrs session 元数据。
- 响应不包含模型消息内容、工具输入输出或 API 密钥，避免暴露持久化文件的敏感内容。
- GUI Conversations 面板新增 Saved aionrs Sessions 区块，展示摘要、模型、消息数、更新时间和 session ID，并支持手动刷新。
- 已完成后端 `cargo check -p aionui-api-types -p aionui-ai-agent -p aionui-app` 与 GUI `RUST_FONTCONFIG_DLOPEN=1 cargo check -p aioncore-gui` 定向编译检查。

## 2026-08-17 数据库结构与用户归属分析

- 已检查 `crates/aionui-db/migrations/001_initial_schema.sql` 至 `039_omp_direct_cli_launch.sql`，整理最终业务表、主键、外键、索引、级联关系和 JSON/软删除字段。
- 已重点核对 `030_user_scope.sql`、`028_project_bind.sql`、`012_assistant_data_unification.sql`、`014_skill_management.sql` 与 `022_cron_execution_dedup.sql`，区分全局资源、用户资源和通过父表继承用户的从属表。
- 已生成 `docs/database-schema.zh-CN.md`，包含数据字典、关系图、`user_id` 可空性矩阵、`system_default_user` 使用边界和维护风险。
- 结论：builtin/internal `agent_metadata`、system builtin `assistant_definitions`、builtin `skills` 使用 `NULL` 表示全局；`folders` 是机器级全局资源且没有 `user_id`；其余用户级字段原则上不得为 NULL，`system_default_user` 仅用于系统初始化和历史单用户兼容回填。
- 本次仅新增/更新 Markdown 文档，未修改数据库 schema、迁移或运行时代码；未执行数据库迁移和 workspace 编译测试。

## 2026-08-17 修复 Project Repository 测试外键失败

- 失败原因：`list_projects_is_scoped_to_owner_and_ordered_by_latest_update` 使用 `other_user` 创建第二个 owner 的 Project，但 migration 030 已为 `projects.user_id` 增加 `REFERENCES users(id)`，测试 fixture 未先创建该用户，SQLite 返回错误 787。
- 修复方式：在测试准备阶段插入合法的 `other_user` 用户记录；未放宽外键约束，也未修改业务实现。
- 验证结果：`cargo nextest run -p aionui-db --test project_repository --no-fail-fast` 通过，10/10 测试通过。

## 2026-08-17 调整 dev AionCore Action

- 将 `.github/workflows/aioncore-dev.yml` 调整为仅在 `dev` 分支变更时构建 Windows x64 后端 `aioncore.exe`。
- 构建目标为 `x86_64-pc-windows-msvc`，使用 `cargo build --release --target ... -p aionui-app`，并上传 `aioncore-windows-x64` Artifact。
- 按要求移除该 workflow 中的 migration 检查、格式检查、Clippy 和 workspace 测试步骤；GUI Windows 构建仍由独立的 `aioncore-gui-windows.yml` 负责。
