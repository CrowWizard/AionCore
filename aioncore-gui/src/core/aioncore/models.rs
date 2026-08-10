use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub build_time: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct WebSocketMessage {
    pub name: String,
    pub data: Value,
}

fn default_optional_data<T>() -> Option<T> {
    None
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(default = "default_optional_data")]
    pub data: Option<T>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ConversationResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub status: Value,
    #[serde(default)]
    pub runtime: Option<Value>,
    #[serde(default)]
    pub r#type: Value,
    #[serde(default)]
    pub extra: Value,
    #[serde(default)]
    pub prompt_capability: Option<PromptCapabilityView>,
}

pub type ConversationListResponse = PaginatedResult<ConversationResponse>;

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MessageResponse {
    pub id: String,
    pub conversation_id: String,
    #[serde(default)]
    pub msg_id: Option<String>,
    #[serde(rename = "type")]
    pub message_type: Value,
    pub content: Value,
    #[serde(default)]
    pub position: Option<Value>,
    #[serde(default)]
    pub status: Option<Value>,
    #[serde(default)]
    pub hidden: bool,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct MessageListResponse {
    pub items: Vec<MessageResponse>,
    #[serde(default)]
    pub oldest_cursor: Option<String>,
    #[serde(default)]
    pub newest_cursor: Option<String>,
    #[serde(default)]
    pub has_more_before: bool,
    #[serde(default)]
    pub has_more_after: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ConversationNameUpdatedPayload {
    pub conversation_id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateConversationRequest {
    pub name: Option<String>,
    pub assistant: AssistantConversationRequest,
    pub extra: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct AssistantConversationRequest {
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateConversationRequest {
    pub name: Option<String>,
    pub name_source: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub files: Vec<ChatFileRef>,
    pub inject_skills: Vec<String>,
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatFileRef {
    Upload { path: String },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SendMessageResponse {
    pub msg_id: String,
    pub turn_id: String,
    pub runtime: ConversationRuntimeSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct CancelConversationRequest {
    pub turn_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct CancelConversationResponse {
    pub runtime: ConversationRuntimeSummary,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ConversationRuntimeSummary {
    pub state: String,
    pub can_send_message: bool,
    pub has_task: bool,
    pub task_status: Option<Value>,
    pub is_processing: bool,
    pub pending_confirmations: usize,
    pub turn_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Confirmation {
    pub id: String,
    pub call_id: String,
    pub title: Option<String>,
    pub action: Option<String>,
    pub description: String,
    pub command_type: Option<String>,
    pub options: Vec<ConfirmationOption>,
    #[serde(default)]
    pub questions: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ConfirmationOption {
    pub label: String,
    pub value: Value,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConfirmRequest {
    pub msg_id: String,
    pub data: Value,
    pub always_allow: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AskAnswerRequest {
    pub answers: Vec<AskQuestionAnswer>,
    pub decline: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct AskQuestionAnswer {
    pub question: String,
    pub labels: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PromptCapabilityView {
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub audio: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SystemSettingsResponse {
    pub language: String,
    pub notification_enabled: bool,
    pub cron_notification_enabled: bool,
    pub command_queue_enabled: bool,
    pub save_upload_to_workspace: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SkillListItemResponse {
    pub name: String,
    pub description: String,
    pub location: String,
    #[serde(default)]
    pub relative_location: Option<String>,
    #[serde(default)]
    pub is_auto_inject: bool,
    pub is_custom: bool,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AionrsSessionResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: String,
    pub summary: String,
    pub message_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AssistantResponse {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub agent_id: String,
    #[serde(default)]
    pub enabled_skills: Vec<String>,
    pub team_selectable: bool,
    #[serde(default)]
    pub team_block_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectListItemResponse {
    pub project_id: String,
    pub name: String,
    pub kind: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectDetailResponse {
    pub project_id: String,
    pub name: String,
    pub explorer: ProjectExplorer,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectExplorer {
    pub workspace_pe_id: String,
    pub entries: Vec<ProjectEntry>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ProjectEntry {
    pub pe_id: String,
    pub role: String,
    pub display_name: Option<String>,
    pub display_path: String,
    pub order_index: i64,
    pub runtime_status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct WorkspaceFlatFileResponse {
    pub name: String,
    pub full_path: String,
    pub relative_path: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TeamMailboxMessageResponse {
    pub id: String,
    pub team_id: String,
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub msg_type: String,
    pub content: String,
    pub summary: Option<String>,
    pub files: Vec<String>,
    pub read: bool,
    pub created_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TeamTaskResponse {
    pub id: String,
    pub team_id: String,
    pub subject: String,
    pub description: Option<String>,
    pub status: String,
    pub owner: Option<String>,
    pub blocked_by: Vec<String>,
    pub blocks: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TeamActivityPageResponse {
    pub items: Vec<TeamActivityItemResponse>,
    pub next_cursor: Option<TeamActivityCursor>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TeamActivityItemResponse {
    pub kind: String,
    pub created_at: i64,
    pub id: String,
    #[serde(default)]
    pub message: Option<TeamMailboxMessageResponse>,
    #[serde(default)]
    pub task: Option<TeamTaskResponse>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct TeamActivityCursor {
    pub ts: i64,
    pub id: String,
}
