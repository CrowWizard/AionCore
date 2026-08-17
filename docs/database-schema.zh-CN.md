# AionCore 数据库结构文档

## 1. 文档范围

本文档依据 `crates/aionui-db/migrations/001_initial_schema.sql` 至
`039_omp_direct_cli_launch.sql` 的最终迁移结果，以及 `aionui-db` 的模型和
Repository 约定整理。文档描述的是 migration 全部执行完成后的 SQLite schema，
不是单个早期 migration 文件中的中间状态。

- 数据库：SQLite，通过 `sqlx` 管理 migration。
- 当前最高 migration：`039`。
- 时间字段：统一使用 Unix epoch milliseconds，除非业务代码另有明确转换。
- JSON 字段：以 `TEXT` 存储，通常由 service 层负责序列化和校验。
- `INTEGER` 布尔字段：`0` 表示 false，`1` 表示 true。
- 数据库启动时会确保 `users.id = 'system_default_user'` 存在，见
  `crates/aionui-db/src/database.rs`。

## 2. 分层关系

```text
users
├── conversations
│   ├── messages
│   ├── conversation_artifacts
│   ├── acp_session
│   └── conversation_assistant_snapshots
├── providers
├── remote_agents
├── mcp_servers
├── oauth_tokens
├── system_settings
├── client_preferences
├── agent_metadata (builtin 可为全局，custom 按用户可见)
├── assistants / assistant_definitions
├── assistant_overrides / assistant_overlays / assistant_preferences
├── skills / skill_import_records
├── assistant_plugins
├── assistant_users / assistant_sessions / assistant_pairing_codes
├── projects / project_explorer
├── teams
│   ├── mailbox
│   └── team_tasks
└── cron_jobs
    └── cron_job_runs

folders 独立于 users，通过 resource_canonical 在机器范围内去重。
```

## 3. 表目录

| 领域 | 表 | 主键 | 归属策略 |
|---|---|---|---|
| 身份 | `users` | `id` | 用户主体；包含系统默认用户 |
| 配置 | `system_settings` | `user_id` | 每用户一行，系统初始化使用默认用户 |
| 配置 | `client_preferences` | `(user_id, key)` | 每用户客户端偏好 |
| AI 配置 | `providers` | `id` | 必须按用户隔离 |
| 会话 | `conversations` | `id` | 必须按用户隔离 |
| 会话 | `messages` | `id` | 通过 conversation 继承用户 |
| 会话 | `conversation_artifacts` | `id` | 通过 conversation 继承用户 |
| 会话 | `acp_session` | `conversation_id` | 通过 conversation 继承用户 |
| AI 配置 | `agent_metadata` | `id`，`agent_id` 唯一 | builtin 全局；custom 带 owner |
| AI 配置 | `assistants` | `id` | 兼容/镜像表，当前列可空但业务应有 owner |
| AI 配置 | `assistant_definitions` | `id` | builtin system 可全局；用户定义按用户 |
| AI 配置 | `assistant_overrides` | `(user_id, assistant_id)` | 每用户覆盖配置 |
| AI 配置 | `assistant_overlays` | `(user_id, assistant_definition_id)` | 每用户覆盖配置 |
| AI 配置 | `assistant_preferences` | `(user_id, assistant_definition_id)` | 每用户使用偏好 |
| 会话 | `conversation_assistant_snapshots` | `conversation_id` | 通过 conversation 继承用户 |
| 远程能力 | `remote_agents` | `id` | 必须按用户隔离 |
| MCP | `mcp_servers` | `id` | 必须按用户隔离，`(user_id, name)` 唯一 |
| MCP | `oauth_tokens` | `(user_id, server_url)` | 必须按用户隔离，属于敏感凭据 |
| 通道 | `assistant_plugins` | `(owner_user_id, id)` | 必须按 owner 隔离 |
| 通道 | `assistant_users` | `id` | 通过 `owner_user_id` 隔离 |
| 通道 | `assistant_sessions` | `id` | 通过 assistant user 继承 owner |
| 通道 | `assistant_pairing_codes` | `(owner_user_id, code)` | 必须按 owner 隔离 |
| 团队 | `teams` | `id` | 必须按用户隔离 |
| 团队 | `mailbox` | `id` | 通过 team 继承用户 |
| 团队 | `team_tasks` | `id` | 通过 team 继承用户 |
| 定时任务 | `cron_jobs` | `id` | 必须按用户隔离 |
| 定时任务 | `cron_job_runs` | `id` | 通过 cron job 继承用户 |
| 技能 | `skills` | `id` | builtin 全局；用户/extension/cron 按用户 |
| 技能 | `skill_import_records` | `id` | 必须按用户隔离 |
| 项目 | `projects` | 内部 `id`；业务 `project_id` 唯一 | 必须按用户隔离 |
| 项目 | `folders` | 内部 `id`；业务 `folder_id` 唯一 | 全局机器资源，不存 user |
| 项目 | `project_explorer` | 内部 `id`；业务 `pe_id` 唯一 | 通过 `owner_user_id` 隔离 |

## 4. 数据字典

### 4.1 身份和配置

#### `users`

用户主体表。`user_type` 为 `local` 或 `aionpro`；local 用户必须有
`password_hash`，系统默认用户使用空密码 hash 表示尚未完成设置。`adopted_by` 和
`adopted_at` 仅用于一次性 AionPro 数据迁移标记。

核心字段：`id`、`user_type`、`external_user_id`、`username`、`email`、
`password_hash`、`avatar_path`、`jwt_secret`、`status`、`session_generation`、
`created_at`、`updated_at`、`last_login`、`adopted_by`、`adopted_at`。

#### `system_settings`

用户级系统设置，最终以 `user_id` 为主键。字段包括语言、通知开关、cron 通知、
命令队列和上传到 workspace 的开关，以及 `updated_at`。

#### `client_preferences`

用户级键值配置。主键为 `(user_id, key)`，`value` 是不透明文本。

#### `providers`

用户的模型供应商配置。`platform`、`base_url`、加密后的 `api_key_encrypted`、
模型列表、能力、健康状态和 `model_settings` 存在同一行。凭据不得在日志或 API
错误中明文输出。

### 4.2 会话和 Agent

#### `conversations`

会话根表。`user_id` 必填并级联删除；`status` 为 `pending`、`running` 或
`finished`。`extra` 保存扩展 JSON，项目绑定阶段增加可空的 `project_id` 和
`folder_id`，历史未解析数据允许延迟回填。

#### `messages`

会话消息流，`conversation_id` 必填并级联删除。`content` 是 JSON 文本；
`position` 支持 `left/right/center/pop`，`status` 支持
`finish/pending/error/work`。`backend_turn_id` 用于可按后端 turn 定位 fork 点，
旧消息或不支持该能力的后端可以为 NULL。

#### `conversation_artifacts`

会话附属产物，种类为 `cron_trigger` 或 `skill_suggest`，状态为
`active/pending/dismissed/saved`。通过会话级联删除。

#### `acp_session`

每个会话最多一行的 Agent session 运行快照。保存 `agent_source`、`agent_id`、
远端 `session_id`、状态、配置及活跃/挂起时间，不重复保存 `user_id`。

#### `agent_metadata`

Agent catalog 和机器级运行状态。`agent_id` 全局唯一。builtin/internal 行的
`user_id` 必须为 NULL，因为 Agent binary、探测结果、能力和机器级启用状态是设备
资源；custom Agent 才写创建者的真实 `user_id`。用户级 enable/disable 不应写到此表，
应写 `assistant_overrides` 或对应 user-scoped 配置。

#### `assistants`

历史兼容/镜像表。新的 Assistant 运行模型以 `assistant_definitions` 为准；该表
仍保留旧的名称、提示词、模型和国际化字段。migration 030 添加的 `user_id` 物理上
可空，但现有数据已回填默认用户，新增业务数据不应继续依赖 NULL。

#### `assistant_definitions`

Assistant 定义主表。`source` 为 `builtin/user/generated`，`owner_type` 为
`system/user`。当 `source = builtin` 且 `owner_type = system` 时 `user_id = NULL`
表示全局定义；其余定义必须绑定真实用户。定义软删除使用 `deleted_at`。

#### `assistant_overrides`、`assistant_overlays`、`assistant_preferences`

三张表分别保存旧 Assistant 覆盖、定义覆盖和最近使用偏好。三者都是用户级数据，
`user_id` 必填，主键包含用户维度，并通过定义表外键级联清理。

#### `conversation_assistant_snapshots`

创建会话时保存 Assistant 的解析结果快照，包括规则、模型、权限、技能、MCP 和
Agent。它只通过 `conversation_id` 关联用户，避免会话历史受后续 Assistant 修改影响。

### 4.3 MCP、远程 Agent 和通道

#### `remote_agents`

用户配置的远程 Agent。包含协议、URL、认证方式、设备密钥/令牌和连接状态；
`user_id` 必填。

#### `mcp_servers`

用户配置的 MCP server。`transport_config`、`original_json` 可能包含敏感配置；
使用 `deleted_at` 软删除，名称唯一性是用户范围而非全局范围。

#### `oauth_tokens`

按用户和 `server_url` 保存 OAuth token。必须使用复合主键，不能恢复为全局
`server_url` 主键，否则会发生用户间凭据覆盖。

#### `assistant_plugins`

通道插件配置。`config` 是加密 JSON，主键是 `(owner_user_id, id)`。

#### `assistant_users`、`assistant_sessions`、`assistant_pairing_codes`

`assistant_users` 是已授权的外部 IM 用户，`assistant_sessions` 是其聊天会话，
`assistant_pairing_codes` 是配对码及状态。三者均属于 `owner_user_id`；其中
`assistant_sessions.user_id` 是外部通道用户 ID，不是 Core 的 `users.id`，不要混淆。

### 4.4 团队和定时任务

#### `teams`

用户级团队定义，包括 workspace、Agent 列表、lead Agent、会话模式和版本。
`user_id` 必填。

#### `mailbox`、`team_tasks`

分别保存团队 Agent 间消息和任务。二者以 `team_id` 关联团队，不重复保存用户归属；
migration 030 通过触发器保证父团队存在。

#### `cron_jobs`、`cron_job_runs`

`cron_jobs` 保存调度定义、payload、执行模式、重试和最近状态；`user_id` 必填。
`cron_job_runs` 保存按 `(job_id, scheduled_at)` 去重的运行租约和结果，通过 job
继承用户，不单独添加 `user_id`。

### 4.5 技能和项目

#### `skills`、`skill_import_records`

`skills` 是技能元数据和软删除状态，实际技能文件在数据目录中；`source = builtin`
时全局行的 `user_id` 为 NULL，其余 source 应绑定用户。`skill_import_records` 是
导入审计记录，`user_id` 必填，`skill_id` 可空以记录导入失败。

#### `projects`、`folders`、`project_explorer`

`projects` 是用户级项目容器；`project_explorer` 将项目与文件夹资源关联，并用
`owner_user_id` 做每用户 workspace 唯一性控制。`folders` 是机器级 canonical
资源身份，按 `resource_canonical` 全局去重，不能写 `system_default_user`。

会话和团队的 `project_id/folder_id` 当前物理可空，NULL 仅表示历史数据未完成解析，
不是“无 owner”的含义；若存在项目引用，服务层必须校验项目 owner 与会话/团队 owner
一致。

## 5. user_id 判定矩阵

以下“字段”同时覆盖命名为 `user_id` 和语义等价的 `owner_user_id`。

| 表 | 当前物理约束 | 允许 NULL | 应写 `system_default_user` | 结论 |
|---|---|---|---|---|
| `users` | 主键 NOT NULL | 否 | 该行本身就是默认用户 | 仅保留固定系统行 |
| `system_settings` | PK/FK NOT NULL | 否 | 系统初始化/未登录旧配置 | 用户登录后使用真实用户 |
| `client_preferences` | PK/FK NOT NULL | 否 | 仅兼容旧全局偏好 | 新请求使用真实用户 |
| `providers` | NOT NULL，历史 DDL 未显式 FK | 否 | 仅旧数据/未登录本地配置 | 新建必须使用真实用户或明确默认上下文 |
| `conversations` | NOT NULL/FK | 否 | 仅系统创建的默认会话 | 正常会话使用调用者用户 |
| `agent_metadata` | 可空/FK | 是 | 不应写 | builtin/internal NULL；custom 写真实用户 |
| `assistants` | 可空/FK | 技术上是 | 仅历史回填兼容 | 新数据不应 NULL，优先真实用户 |
| `assistant_definitions` | 可空/FK | 是 | 不应写来代替 NULL | builtin system NULL；用户/生成定义写真实用户 |
| `assistant_overrides` | NOT NULL/FK | 否 | 仅默认用户的全局兼容覆盖 | 真实用户设置写真实用户 |
| `assistant_overlays` | NOT NULL/FK | 否 | 仅默认模板覆盖 | 真实用户覆盖写真实用户 |
| `assistant_preferences` | NOT NULL/FK | 否 | 仅默认用户历史偏好 | 真实用户偏好写真实用户 |
| `skills` | 可空/FK | 是 | 不应写来代替 NULL | builtin NULL；其他 source 写真实用户 |
| `skill_import_records` | NOT NULL/FK | 否 | 旧/系统导入兼容 | 按导入发起用户写入 |
| `assistant_plugins` | NOT NULL/FK | 否 | 旧单用户部署兼容 | 新插件必须写 owner 真实用户 |
| `assistant_users` | NOT NULL/FK | 否 | 旧单用户通道兼容 | 新授权必须写 owner 真实用户 |
| `assistant_pairing_codes` | NOT NULL/FK | 否 | 旧单用户配对兼容 | 新配对必须写 owner 真实用户 |
| `remote_agents` | NOT NULL，历史 DDL 未显式 FK | 否 | 旧配置迁移已使用 | 新配置必须按真实用户隔离 |
| `mcp_servers` | NOT NULL/FK | 否 | 旧全局 MCP 迁移已使用 | 新配置必须按真实用户隔离 |
| `oauth_tokens` | NOT NULL/FK | 否 | 旧 token 迁移已使用 | 必须严格按真实用户隔离 |
| `projects` | NOT NULL/FK | 否 | 旧项目迁移已使用 | 新项目写真实用户 |
| `project_explorer` | NOT NULL/FK | 否 | 旧项目迁移已使用 | 必须与 project owner 一致 |
| `teams` | NOT NULL，历史 DDL 默认值 | 否 | 仅系统/旧团队 | 用户创建团队写真实用户 |
| `folders` | 无此字段 | 不适用 | 不适用 | 全局机器资源，禁止新增 owner |

不直接保存 `user_id` 的从属表：`messages`、`conversation_artifacts`、`acp_session`、
`conversation_assistant_snapshots`、`mailbox`、`team_tasks`、`cron_job_runs`、
`assistant_sessions`。这些表必须通过父记录解析 owner；不能为了方便写入一个脱离父级
的 `system_default_user`。

## 6. 写入规则建议

1. 请求上下文已有认证用户时，所有用户级资源使用认证用户的真实 `users.id`。
2. `system_default_user` 只代表本地单用户兼容/系统初始化上下文，不代表“未知用户”。
3. 对全局机器资源使用 NULL 表达全局：当前明确适用的是 builtin/internal
   `agent_metadata`、system builtin `assistant_definitions` 和 builtin `skills`。
4. 子表先查询父表 owner，再使用父表 owner 执行读写；禁止从请求参数独立拼接 owner。
5. 用户归属迁移必须同时检查父子 owner 一致性、唯一索引冲突和凭据隔离，不能只批量
   UPDATE `user_id`。
6. 新增表若资源可被多个用户看到，应优先采用“全局定义 + 用户 overlay/preferences”
   模式，而不是复制全局定义到 `system_default_user`。

## 7. 已知结构风险和维护点

- `providers`、`remote_agents`、`teams`、`cron_jobs` 等字段在最终迁移中虽然是
  `NOT NULL`，部分仍缺少对 `users(id)` 的显式 FK；Repository 和 service 必须承担
  owner 合法性校验。
- `assistants.user_id` 和 `assistant_definitions.user_id` 的可空性是有意不同的：
  前者是兼容遗留表，后者用 NULL 表示真正的全局 builtin 定义。
- `folders` 的全局去重是机器资源语义；若未来支持远程/租户资源，应新建资源 scope
  模型，不应直接给现有 `folders` 填默认用户。
- migration 030 中的 `system_default_user` 回填是历史兼容动作，不应作为新代码的
  默认 owner 传播到已认证用户的数据。

## 8. 依据

- `crates/aionui-db/migrations/001_initial_schema.sql`
- `crates/aionui-db/migrations/002_legacy_data_normalize.sql`
- `crates/aionui-db/migrations/012_assistant_data_unification.sql`
- `crates/aionui-db/migrations/014_skill_management.sql`
- `crates/aionui-db/migrations/022_cron_execution_dedup.sql`
- `crates/aionui-db/migrations/028_project_bind.sql`
- `crates/aionui-db/migrations/030_user_scope.sql`
- `crates/aionui-db/migrations/031_add_omp_builtin_acp_agent.sql`
- `crates/aionui-db/migrations/032_adoption_once_marker.sql`
- `crates/aionui-db/migrations/036_conversation_fork.sql`
- `crates/aionui-db/src/database.rs`
- `crates/aionui-db/src/models/*.rs`
- `crates/aionui-db/src/repository/*.rs`
