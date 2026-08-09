# AionCore GPUI Desktop Client Plan

## 目标

基于 `aioncore-gui/` 中复制的 Agent Studio GPUI 桌面应用，构建 AionCore 的独立桌面客户端。客户端通过 HTTP REST API 和 WebSocket 访问 AionCore，AionCore 保持为唯一的 agent 生命周期、权限、消息持久化和 MCP/Team 业务所有者。

首期目标是完成单聊闭环：服务连接、会话列表、创建会话、历史消息、发送消息、流式回复、取消会话，以及错误和重连状态展示。

## 已建立的目录

- `aioncore-gui/`：从 `agent-studio/` 复制的独立 GPUI 工程。
- `.kilo/plans/aioncore-gui/`：本实施计划。

已复制的内容：

- Cargo workspace 配置、锁文件、Rust toolchain 与 gitignore。
- `src/`：窗口、Dock、面板、组件、主题初始化、HTTP 客户端等 UI 代码。
- `crates/`：当前 Agent Studio 的内部 crates，作为逐步替换的参考与过渡代码。
- `assets/`、`themes/`、`locales/`：界面资源、主题和国际化文本。

未复制的内容：

- `.git/`、`.github/`、`.claude/` 等源仓库元数据。
- `config.json`、`config.example.json`、`config.test.json`，避免复制本地 agent 命令、环境变量或潜在密钥。
- `target/`、`sessions/` 等构建和运行数据。

## 已验证的契约

AionCore 的完整服务入口是 `aioncore`，由 `crates/aionui-app/src/main.rs` 启动。默认服务端口为 `127.0.0.1:25808`，本地桌面使用应以 `--local` 模式启动。

`--local` 模式会为受保护的 API 注入 `system_default_user`，跳过 JWT 校验，并启用面向本地客户端的 CORS。依据：

- `crates/aionui-app/src/cli.rs`
- `crates/aionui-app/src/bootstrap/environment.rs`
- `crates/aionui-auth/src/middleware.rs`
- `crates/aionui-app/src/router/routes.rs`

单聊 API：

- `GET /health`
- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/{id}`
- `PATCH /api/conversations/{id}`
- `DELETE /api/conversations/{id}`
- `GET /api/conversations/{id}/messages`
- `POST /api/conversations/{id}/messages`
- `POST /api/conversations/{id}/cancel`
- `POST /api/conversations/{id}/runtime/ensure`
- `GET /api/conversations/{id}/confirmations`
- `POST /api/conversations/{id}/confirmations/{callId}/confirm`
- `POST /api/conversations/{id}/asks/{requestId}/answer`

实时接口：

- `GET /ws` 升级为 WebSocket。
- 事件信封格式：`{ "name": "...", "data": { ... } }`。
- 首期重点处理 `message.stream`、`message.userCreated`、`conversation.listChanged`、`conversation.nameUpdated`、`confirmation.remove`。

消息历史以 `MessageResponse` 为准；流式消息由 `message.stream` 推送。GUI 不应再将 ACP `SessionUpdate` 作为自身与 AionCore 的通信协议。

## 目标架构

```text
AionCore GUI (GPUI)
  |
  | HTTP REST + WebSocket
  v
CoreClient
  |
  v
AionCore server
  |
  +-- SQLite: conversation/message/settings/provider/MCP/team
  +-- agent runtime: ACP/direct CLI/aionrs
  +-- permissions, confirmations, skills, file/project services
```

GUI 只维护渲染和短期交互状态。服务端是以下数据的唯一事实来源：

- 对话、消息、流式状态和持久化历史。
- Agent 发现、启动、停止、运行时状态。
- Provider、MCP、Skill、Assistant、Team 和文件配置。
- 需要用户确认或回答的问题。

## 迁移边界

### 可直接复用

- GPUI 应用初始化、窗口、菜单、标题栏、系统托盘、Dock 和主题。
- 通用组件：聊天输入框、文件选择器、状态指示器、命令建议、工具调用详情 UI。
- 会话、编辑器、终端、任务和设置面板的布局与交互外壳。
- `reqwest_client` 的 Rustls、代理和 HTTP 基础设施。
- 国际化和图标资源。

### 必须替换

- `agentx-agent` 的 `AgentManager` 和所有本地 ACP 子进程创建。
- `AgentService` 中基于 ACP 的 session 创建、恢复、列举和 prompt 发送。
- `MessageService` 的 ACP EventHub 输入与本地 JSONL 持久化。
- `PersistenceService` 的 `sessions/*.jsonl` 读写。
- `config.json` 驱动的 agent、model、MCP、command 配置服务。
- `main.rs` 中读取 Agent Studio 配置并初始化本地 agent 的路径。

### 需要转换而非直接复用

- 当前 `ConversationPanel` 消费 `agent_client_protocol::schema::SessionUpdate`。
- AionCore 使用 REST `MessageResponse` 和 WebSocket `message.stream`。
- 首期应新增 AionCore 专用消息视图模型和 reducer。不要将服务端 JSON 硬转换为 ACP `SessionUpdate`，除非经实际 payload 捕获证明字段和语义完全对应。

## 实施阶段

### 阶段 0：工程基线

1. 将包名、二进制名、应用显示名从 `agentx` 改为 `aioncore-gui`，保留独立 Cargo workspace。
2. 新增仅包含客户端需要依赖的 `aioncore-client` crate，或先在主 crate 的 `src/core/aioncore/` 下建立模块；根据首期代码规模在实现时决定。
3. 从 workspace 依赖中移除生产代码不再使用的 ACP agent manager crates，保留 UI 必要类型的最小集合。
4. 新增 GUI 配置文件，仅保存 `backend_url`、是否自动拉起后端、`data_dir`、`work_dir`、窗口和主题偏好；不得保存 Provider API Key。
5. 支持显式 `--backend-url` 覆盖配置，默认 `http://127.0.0.1:25808`。

验收：`cargo check` 可执行，应用能够启动至空壳窗口，且不启动任何 agent CLI。

### 阶段 1：CoreClient 与连接生命周期

1. 定义 AionCore API DTO，优先直接复用或镜像 `aionui-api-types` 的序列化字段，不将 GUI crate 直接接入后端 workspace。
2. 实现 `CoreClient`：健康检查、通用 JSON 请求、统一错误映射和请求超时。
3. 实现 WebSocket 客户端：连接、断线指数退避、显式关闭、按事件名分派 JSON payload。
4. 通过 GUI 内部 EventHub 发布与业务无关的客户端事件，例如 connected、disconnected、request_failed、websocket_event。
5. 在标题栏或状态栏显示 Core 连接状态，不显示敏感响应内容。

验收：连接到 `aioncore --local` 后 `/health` 成功；停止后端后 GUI 可显示断开并重连；恢复后端后 GUI 自动恢复连接。

观测性：客户端只记录连接状态、HTTP 方法、路径、状态码、事件名和重连次数。禁止记录 prompt、消息正文、工具参数、token 和响应正文。

### 阶段 2：会话列表与单聊历史

1. 建立 `ConversationStore`，以 AionCore `conversation_id` 替代 Agent Studio `session_id`。
2. 会话侧栏调用 `GET /api/conversations`，支持初次加载、刷新、创建和删除。
3. 单聊面板选择会话后调用 `GET /api/conversations/{id}` 与 `GET /api/conversations/{id}/messages`。
4. 以 AionCore 的 `MessageResponse` 渲染历史消息，正确处理文本、思考、工具调用、错误、隐藏消息和 artifact 相关内容。
5. 将 `conversation.listChanged` 与 `conversation.nameUpdated` 合并至 Store，发生不确定状态时以 REST 刷新收敛。

验收：新建、重命名、删除会话后列表与服务端一致；重启 GUI 后历史从 AionCore SQLite 恢复，不依赖本地 JSONL。

### 阶段 3：发送与流式消息

1. 聊天输入改为 `POST /api/conversations/{id}/messages`，请求体使用 `content`、`files`、`inject_skills`、`hidden`。
2. 发送成功后依据返回的 `msg_id`、`turn_id` 和 runtime 状态建立本地临时 turn 状态。
3. 新增 `MessageStreamReducer`，按 `conversation_id` 和 `turn_id` 消费 `message.stream`。
4. 先实现已被代码和 live payload 验证的 text/content、thinking、tool call、error、turn finish 等事件；未知 event type 仅记录事件名并保留 UI 可用性。
5. 对收到的服务端持久化事件进行去重，避免 POST 乐观 UI、WebSocket 和刷新历史三路重复。
6. 实现 `POST /api/conversations/{id}/cancel`，在运行态显示取消按钮与等待态。

验收：用户发送一条消息后，文本流、思考、工具调用与完成态能连续渲染；重连或切换会话不丢失已持久化消息；取消操作反映服务端 runtime 状态。

测试要求：用实际启动的 AionCore 加一个能产生流式文本、工具调用、用户确认和错误的 agent 场景捕获 payload。不能以仅有 start/text/finish 的单一 happy-path 证明通用兼容性。

### 阶段 4：确认、问题与附件

1. 实现 confirmation 列表和 confirm 操作，映射各选项为 GPUI Dialog。
2. 实现 AskUserQuestion 渲染与 `asks/{requestId}/answer` 回答。
3. 通过 AionCore `/api/fs/upload` 或已验证的聊天附件接口上传文件，发送时引用后端返回的 `ChatFileRef`。
4. 对无原生媒体能力的 agent，遵循会话 detail 的 `prompt_capability` 展示退化状态。

验收：确认卡、问题卡、取消和文件附件均经过服务端 API 闭环；刷新 GUI 后 pending confirmation 不丢失。

### 阶段 5：设置、项目与 Team

1. 设置面板按 AionCore 的 `/api/settings`、`/api/providers`、`/api/mcp`、`/api/skills` 和 assistant/agent API 重做数据源。
2. 不向 GUI 传回已加密或敏感的 Provider 凭证；编辑请求直接提交给 AionCore。
3. 文件、项目和编辑器面板接 `/api/fs` 与 `/api/projects`，由服务端统一限制工作区边界。
4. Team 面板接 `/api/teams`、成员、run state、activity、task、mailbox 及 `team.*` 事件。
5. 仅在单聊稳定后扩展 Team，避免把多 agent 生命周期问题混入基础连接层。

验收：设置修改经 AionCore 持久化；文件操作遵守服务端权限；Team 状态和活动通过 REST + WebSocket 收敛。

## 首期文件改造清单

新增：

- `aioncore-gui/src/core/aioncore/client.rs`
- `aioncore-gui/src/core/aioncore/models.rs`
- `aioncore-gui/src/core/aioncore/websocket.rs`
- `aioncore-gui/src/core/aioncore/events.rs`
- `aioncore-gui/src/core/aioncore/conversation_store.rs`
- `aioncore-gui/src/core/aioncore/message_reducer.rs`
- `aioncore-gui/src/core/aioncore/mod.rs`

优先修改：

- `aioncore-gui/src/main.rs`
- `aioncore-gui/src/app/app_state.rs`
- `aioncore-gui/src/app/service_registry.rs`
- `aioncore-gui/src/panels/conversation/panel.rs`
- `aioncore-gui/src/panels/session_manager.rs`
- `aioncore-gui/src/panels/settings_panel/*`
- `aioncore-gui/src/workspace/*`
- `aioncore-gui/Cargo.toml`

计划删除或从生产路径移除：

- `aioncore-gui/crates/agentx-agent/`
- `aioncore-gui/crates/agentx-services/src/agent_service.rs`
- `aioncore-gui/crates/agentx-services/src/message_service.rs`
- `aioncore-gui/crates/agentx-services/src/persistence_service.rs`
- 依赖本地 `config.json` 的配置管理逻辑。

删除动作必须在替代服务已接入、调用点已迁移并通过验证后执行。

## 验证顺序

每个阶段完成后执行：

```bash
cargo fmt --all -- --check
cargo check
cargo test
cargo clippy -- -D warnings
```

需要服务端联调时，在 AionCore 根目录启动：

```bash
cargo run -p aionui-app --bin aioncore -- --local --data-dir ./data --work-dir ./workspace
```

然后在 `aioncore-gui/` 目录启动 GUI：

```bash
cargo run -- --backend-url http://127.0.0.1:25808
```

在首期完成前，至少执行这些手工可观察场景：

1. 后端未运行时启动 GUI。
2. 后端运行后 GUI 自动连接。
3. 创建、切换、重命名、删除会话。
4. 正常文本流式回复。
5. 工具调用和工具结果。
6. 思考块开始、增量、完成。
7. 用户确认和 AskUserQuestion。
8. 取消长请求。
9. WebSocket 断线后重连与历史收敛。
10. 重启 GUI 后消息历史正确恢复。

## 不做的事项

- 不在 GUI 内直接创建 ACP session 或启动 Claude/Codex/Gemini 等 agent 子进程。
- 不维护第二份消息历史或 agent 配置真相源。
- 不把 AionCore 的 SQLite 文件直接作为 GUI 数据库访问。
- 不在首期实现 Team、Provider 编辑、MCP 编辑、Git Worktree 或自动更新。
- 不把 GPUI 直接并入 AionCore 后端 Cargo workspace。

## 主要风险和缓解措施

| 风险 | 影响 | 缓解措施 |
| --- | --- | --- |
| AionCore 流事件与 AgentX ACP 事件模型不同 | 聊天 UI 不能直接复用数据输入 | 编写专用 DTO/reducer，基于真实 WebSocket payload 逐类映射 |
| GPUI Git 依赖构建慢或发生 API 漂移 | 开发环境不稳定 | 固定 `Cargo.lock`，只在明确需要时升级依赖 |
| 两套 agent 管理逻辑同时存在 | 重复 agent、状态错乱、历史分叉 | 阶段 0 即切断 GUI 本地 agent 启动路径 |
| local 模式的无认证访问被暴露到局域网 | 本地数据与文件风险 | 默认固定 loopback；只有明确配置时才允许非 loopback host |
| Team 和文件模块范围膨胀 | 基础单聊延期 | 严格按阶段推进，单聊闭环通过后再接入扩展能力 |
