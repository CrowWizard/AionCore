use gpui::{
    App, ClipboardEntry, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, ScrollHandle,
    SharedString, Styled, Window, div, prelude::*, px,
};

use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Selectable, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::InputState,
    menu::{DropdownMenu as _, PopupMenuItem},
    select::SelectState,
    skeleton::Skeleton,
    spinner::Spinner,
    v_flex,
};

// Use the published ACP schema crate
use crate::core::aioncore::{
    AcpConfigOption, AskAnswerRequest, AskQuestionAnswer, AssistantResponse, Confirmation, ConversationRuntimeSummary,
    MessageView, SlashCommand,
};
use crate::utils::clipboard::ClipboardImage;
use crate::{
    AppState, ChatInputBox,
    app::actions::AddCodeSelection,
    components::{ModeSelectItem, ModelSelectItem, render_message},
    core::services::SessionStatus,
    panels::dock_panel::DockPanel,
};
use agent_client_protocol::schema::AvailableCommand;
use chrono::{DateTime, Utc};
use rust_i18n::t;

/// Session status information for display
#[derive(Clone, Debug)]
pub struct SessionStatusInfo {
    pub agent_name: String,
    pub status: SessionStatus,
    pub last_active: DateTime<Utc>,
    pub message_count: usize,
}

/// Conversation panel backed by AionCore REST and WebSocket state.
pub struct ConversationPanel {
    focus_handle: FocusHandle,
    /// AionCore conversation ID.
    session_id: Option<String>,
    /// Scroll handle for auto-scrolling to bottom
    scroll_handle: ScrollHandle,
    /// Input state for the chat input box
    input_state: Entity<InputState>,
    /// Clipboard images materialized as controlled temporary PNG files.
    pasted_images: Vec<ClipboardImage>,
    ask_selections: std::collections::HashMap<String, Vec<String>>,
    expanded_messages: std::collections::HashSet<String>,
    /// List of code selections from editor
    code_selections: Vec<AddCodeSelection>,
    history_messages: Vec<MessageView>,
    confirmations: Vec<Confirmation>,
    prompt_capability: Option<(bool, bool)>,
    active_turn_id: Option<String>,
    /// Session status information for display
    session_status: Option<SessionStatusInfo>,
    /// Workspace information
    workspace_id: Option<String>,
    workspace_name: Option<String>,
    working_directory: Option<String>,
    is_active: bool,
    agent_icon: Option<IconName>,
    agent_name: Option<String>,
    conversation_title: Option<String>,
    draft_assistants: Vec<AssistantResponse>,
    draft_assistant_id: Option<String>,
    draft_workspace: Option<std::path::PathBuf>,
    config_options: Vec<AcpConfigOption>,
    pending_config_options: std::collections::HashSet<String>,
    slash_commands: Vec<SlashCommand>,
}

const AUTO_SCROLL_THRESHOLD_PX: f32 = 120.0;

fn selected_agent_icon(state: &crate::core::aioncore::ConversationState) -> Option<IconName> {
    state
        .selected_conversation
        .as_ref()
        .and_then(|conversation| conversation.assistant.as_ref())
        .map(|assistant| match assistant.backend.to_ascii_lowercase().as_str() {
            value if value.contains("codex") => IconName::SquareTerminal,
            value if value.contains("claude") => IconName::Bot,
            value if value.contains("aion") => IconName::Bot,
            _ => IconName::Bot,
        })
}

fn normalized_tab_title(value: &str) -> String {
    let title = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = title.chars();
    let shortened = chars.by_ref().take(32).collect::<String>();
    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}

fn workspace_from_extra(extra: &serde_json::Value) -> Option<std::path::PathBuf> {
    extra
        .get("workspace")
        .and_then(serde_json::Value::as_str)
        .map(std::path::PathBuf::from)
}

#[derive(Clone)]
struct AskQuestion {
    text: String,
    labels: Vec<String>,
    multi_select: bool,
}

fn parse_single_ask_question(value: &serde_json::Value) -> Option<AskQuestion> {
    let questions = value.as_array()?;
    if questions.len() != 1 {
        return None;
    }

    let question = questions.first()?.as_object()?;
    let text = question.get("question")?.as_str()?.to_owned();
    let labels = question
        .get("options")?
        .as_array()?
        .iter()
        .map(|option| option.get("label")?.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    if labels.is_empty() {
        return None;
    }

    Some(AskQuestion {
        text,
        labels,
        multi_select: question
            .get("multiSelect")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

fn confirmation_element_id(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn ask_answer_request(question: String, labels: Vec<String>) -> AskAnswerRequest {
    AskAnswerRequest {
        answers: vec![AskQuestionAnswer { question, labels }],
        decline: false,
    }
}

fn answer_button(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    store: crate::core::aioncore::ConversationStore,
    conversation_id: String,
    request_id: String,
    confirmation_id: String,
    request: AskAnswerRequest,
    disabled: bool,
) -> Button {
    Button::new(id)
        .label(label)
        .disabled(disabled)
        .on_click(move |_, _, cx| {
            let store = store.clone();
            let conversation_id = conversation_id.clone();
            let request_id = request_id.clone();
            let confirmation_id = confirmation_id.clone();
            let request = request.clone();
            cx.spawn(async move |cx| {
                if let Err(error) = store
                    .answer_ask(&conversation_id, &request_id, request, &confirmation_id)
                    .await
                {
                    log::warn!("AionCore ask answer failed: {error:?}");
                }
                let _ = cx.update(|_| {});
            })
            .detach();
        })
}

fn decline_button(
    confirmation_id: String,
    store: crate::core::aioncore::ConversationStore,
    conversation_id: String,
    request_id: String,
) -> Button {
    answer_button(
        ("ask-decline", confirmation_element_id(&confirmation_id)),
        "Decline",
        store,
        conversation_id,
        request_id,
        confirmation_id,
        AskAnswerRequest {
            answers: Vec::new(),
            decline: true,
        },
        false,
    )
}

impl ConversationPanel {
    /// Create a new panel with mock data (for demo purposes)
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        log::info!("🚀 Creating ConversationPanel view");
        let entity = cx.new(|cx| Self::new(window, cx));
        Self::subscribe_to_code_selections(&entity, cx);
        log::info!("✅ ConversationPanel view created and subscribed");
        entity
    }

    pub fn view_for_workspace(workspace: std::path::PathBuf, window: &mut Window, cx: &mut App) -> Entity<Self> {
        let entity = Self::view(window, cx);
        entity.update(cx, |panel, cx| {
            panel.draft_workspace = Some(workspace);
            cx.notify();
        });
        entity
    }

    /// Create a new panel for a specific session (no mock data)
    pub fn view_for_session(session_id: String, window: &mut Window, cx: &mut App) -> Entity<Self> {
        log::info!("🚀 Creating ConversationPanel for session: {}", session_id);
        let entity = cx.new(|cx| Self::new_for_session(session_id.clone(), window, cx));

        // Load historical messages before subscribing to new updates
        Self::load_history_for_session(&entity, session_id.clone(), cx);
        Self::load_session_capabilities(&entity, session_id.clone(), cx);

        Self::subscribe_to_stream_events(&entity, session_id.clone(), cx);
        Self::subscribe_to_code_selections(&entity, cx);
        log::info!("✅ ConversationPanel created for session: {}", session_id);
        entity
    }

    pub fn session_id(&self) -> Option<String> {
        self.session_id.clone()
    }

    pub fn tab_title(&self) -> String {
        self.conversation_title
            .clone()
            .unwrap_or_else(|| "New conversation".to_owned())
    }

    /// Get the workspace_id (if available)
    pub fn workspace_id(&self) -> Option<String> {
        self.workspace_id.clone()
    }

    /// Get the workspace_name (if available)
    pub fn workspace_name(&self) -> Option<String> {
        self.workspace_name.clone()
    }

    /// Get the working_directory (if available)
    pub fn working_directory(&self) -> Option<String> {
        self.working_directory.clone()
    }

    pub fn effective_workspace(&self) -> Option<std::path::PathBuf> {
        self.working_directory
            .as_deref()
            .map(std::path::PathBuf::from)
            .or_else(|| self.draft_workspace.clone())
    }

    fn update_active_workspace(&self, cx: &mut App) {
        if self.is_active {
            let workspace_state = AppState::global(cx).active_conversation_workspace.clone();
            let workspace = self.effective_workspace();
            workspace_state.update(cx, |current, cx| {
                if *current == workspace {
                    return;
                }
                *current = workspace;
                cx.notify();
            });
        }
    }

    fn apply_config_options(&mut self, options: Vec<AcpConfigOption>) {
        clear_confirmed_options(&mut self.pending_config_options, &options);
        self.config_options = options;
    }

    fn apply_config_stream_update(&mut self, data: &serde_json::Value) {
        match data.get("type").and_then(serde_json::Value::as_str) {
            Some("acp_config_option") => {
                if let Some(options) = data
                    .get("data")
                    .and_then(|value| value.get("config_options"))
                    .cloned()
                    .and_then(|value| serde_json::from_value::<Vec<AcpConfigOption>>(value).ok())
                {
                    self.apply_config_options(options);
                }
            }
            Some("acp_mode_info") => {
                let Some(mode) = data
                    .get("data")
                    .and_then(|value| value.get("current_mode_id"))
                    .and_then(serde_json::Value::as_str)
                else {
                    return;
                };
                if let Some(option) = self.config_options.iter_mut().find(|option| option.id == "mode") {
                    option.current_value = Some(mode.to_owned());
                    clear_confirmed_options(&mut self.pending_config_options, &self.config_options);
                }
            }
            _ => {}
        }
    }

    fn load_session_capabilities(entity: &Entity<Self>, conversation_id: String, cx: &mut App) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let weak_entity = entity.downgrade();
        cx.spawn(async move |cx| {
            let runtime = store.ensure_runtime(&conversation_id).await;
            let commands = store.list_slash_commands(&conversation_id).await;
            let _ = cx.update(|cx| {
                let Some(entity) = weak_entity.upgrade() else {
                    return;
                };
                entity.update(cx, |this, cx| {
                    if let Ok(runtime) = runtime {
                        this.apply_config_options(runtime.config_options);
                        this.session_status = Self::runtime_status(Some(&runtime.runtime));
                    } else if let Err(error) = runtime {
                        log::debug!("Unable to load ACP config options: {error:?}");
                    }
                    if let Ok(commands) = commands {
                        this.slash_commands = commands;
                    } else if let Err(error) = commands {
                        log::debug!("Unable to load ACP slash commands: {error:?}");
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn set_config_option(&mut self, option_id: String, value: String, cx: &mut Context<Self>) {
        let Some(conversation_id) = self.session_id.clone() else {
            return;
        };
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        self.pending_config_options.insert(option_id.clone());
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = store.set_config_option(&conversation_id, &option_id, value).await;
            let _ = cx.update(|cx| {
                let Some(entity) = entity.upgrade() else {
                    return;
                };
                entity.update(cx, |this, cx| match result {
                    Ok(response) => {
                        if let Some(options) = response.config_options {
                            this.apply_config_options(options);
                        }
                        if response.confirmation != "pending_next_turn" {
                            this.pending_config_options.remove(&option_id);
                        }
                        cx.notify();
                    }
                    Err(error) => {
                        this.pending_config_options.remove(&option_id);
                        log::warn!("Failed to update ACP config option {option_id}: {error:?}");
                        cx.notify();
                    }
                });
            });
        })
        .detach();
    }

    fn new(window: &mut Window, cx: &mut App) -> Self {
        log::info!("🔧 Initializing ConversationPanel (new)");
        Self::new_internal(None, window, cx)
    }

    fn new_for_session(session_id: String, window: &mut Window, cx: &mut App) -> Self {
        log::info!("🔧 Initializing ConversationPanel for session: {}", session_id);
        Self::new_internal(Some(session_id), window, cx)
    }

    fn new_internal(session_id: Option<String>, window: &mut Window, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();
        let input_state = Self::create_input_state(window, cx);
        let draft_assistants = AppState::global(cx)
            .settings_store()
            .map(|store| {
                store
                    .snapshot()
                    .assistants
                    .into_iter()
                    .filter(|assistant| assistant.enabled)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let draft_assistant_id = draft_assistants.first().map(|assistant| assistant.id.clone());
        Self {
            focus_handle,
            session_id,
            scroll_handle,
            input_state,
            pasted_images: Vec::new(),
            ask_selections: std::collections::HashMap::new(),
            expanded_messages: std::collections::HashSet::new(),
            code_selections: Vec::new(),
            history_messages: Vec::new(),
            confirmations: Vec::new(),
            prompt_capability: None,
            active_turn_id: None,
            session_status: None,
            workspace_id: None,
            workspace_name: None,
            working_directory: None,
            is_active: false,
            agent_icon: None,
            agent_name: None,
            conversation_title: None,
            draft_assistants,
            draft_assistant_id,
            draft_workspace: None,
            config_options: Vec::new(),
            pending_config_options: std::collections::HashSet::new(),
            slash_commands: Vec::new(),
        }
    }

    fn create_input_state(window: &mut Window, cx: &mut App) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(1, 3)
                .soft_wrap(true)
                .submit_on_enter(true)
                .placeholder("Type a message...")
        })
    }

    fn should_auto_scroll(&self) -> bool {
        let max_offset = self.scroll_handle.max_offset().y;
        let offset = self.scroll_handle.offset().y;
        let distance_to_bottom = max_offset + offset;
        distance_to_bottom <= px(AUTO_SCROLL_THRESHOLD_PX)
    }

    fn render_history_message(
        &self,
        message: &MessageView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let message_id = message.id.clone();
        let expanded = self.expanded_messages.contains(&message_id);
        render_message(
            message,
            expanded,
            cx.listener(move |this, _, _, cx| {
                if !this.expanded_messages.insert(message_id.clone()) {
                    this.expanded_messages.remove(&message_id);
                }
                cx.notify();
            }),
            window,
            cx,
        )
    }

    pub fn load_history_for_session(entity: &Entity<Self>, conversation_id: String, cx: &mut App) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            log::warn!("AionCore conversation store is not initialized");
            return;
        };
        let weak_entity = entity.downgrade();
        cx.spawn(async move |cx| {
            let result = store.load_conversation(&conversation_id).await;
            let _ = cx.update(|cx| {
                if let Some(entity) = weak_entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        if let Err(error) = &result {
                            log::warn!("Failed to load conversation history: {error:?}");
                        }
                        let message_state = store.message_snapshot();
                        this.history_messages = message_state.messages;
                        this.confirmations = store.snapshot().confirmations;
                        this.prompt_capability = store
                            .snapshot()
                            .selected_conversation
                            .as_ref()
                            .and_then(|conversation| conversation.prompt_capability.as_ref())
                            .map(|capability| (capability.image, capability.audio));
                        let snapshot = store.snapshot();
                        this.agent_icon = selected_agent_icon(&snapshot);
                        this.agent_name = snapshot
                            .selected_conversation
                            .as_ref()
                            .and_then(|conversation| conversation.assistant.as_ref())
                            .map(|assistant| assistant.name.clone());
                        this.conversation_title = snapshot
                            .selected_conversation
                            .as_ref()
                            .map(|conversation| normalized_tab_title(&conversation.name));
                        this.working_directory = snapshot
                            .selected_conversation
                            .as_ref()
                            .and_then(|conversation| workspace_from_extra(&conversation.extra))
                            .and_then(|workspace| workspace.into_os_string().into_string().ok());
                        this.update_active_workspace(cx);
                        this.active_turn_id = message_state.active_turn_id;
                        this.scroll_handle.scroll_to_bottom();
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn subscribe_to_stream_events(entity: &Entity<Self>, conversation_id: String, cx: &mut App) {
        let Some(client) = AppState::global(cx).core_client().cloned() else {
            return;
        };
        let mut stream_events = client.events().subscribe_websocket_event("message.stream");
        let mut user_events = client.events().subscribe_websocket_event("message.userCreated");
        let mut confirmation_events = client.events().subscribe_websocket_event("confirmation.remove");
        let weak_entity = entity.downgrade();
        cx.spawn(async move |cx| {
            loop {
                tokio::select! {
                    event = stream_events.recv() => {
                        let Ok(data) = event else { break };
                        if data.get("conversation_id").and_then(serde_json::Value::as_str) != Some(conversation_id.as_str()) {
                            continue;
                        }
                        let _ = cx.update(|cx| {
                            if let Some(entity) = weak_entity.upgrade() {
                                entity.update(cx, |this, cx| {
                                    if let Some(store) = AppState::global(cx).conversation_store() {
                                        this.apply_config_stream_update(&data);
                                        let _ = store.apply_websocket_event("message.stream", &data);
                                        let state = store.message_snapshot();
                        this.history_messages = state.messages;
                        this.confirmations = store.snapshot().confirmations;
                        this.active_turn_id = state.active_turn_id;
                                        this.session_status = Self::runtime_status(state.runtime.as_ref());
                                        let snapshot = store.snapshot();
                                        this.agent_icon = selected_agent_icon(&snapshot);
                                        this.agent_name = snapshot
                                            .selected_conversation
                                            .as_ref()
                                            .and_then(|conversation| conversation.assistant.as_ref())
                                            .map(|assistant| assistant.name.clone());
                                        this.scroll_handle.scroll_to_bottom();
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    }
                    event = confirmation_events.recv() => {
                        let Ok(data) = event else { break };
                        if data.get("conversation_id").and_then(serde_json::Value::as_str) != Some(conversation_id.as_str()) {
                            continue;
                        }
                        let _ = cx.update(|cx| {
                            if let Some(entity) = weak_entity.upgrade() {
                                entity.update(cx, |this, cx| {
                                    if let Some(store) = AppState::global(cx).conversation_store() {
                                        let _ = store.apply_websocket_event("confirmation.remove", &data);
                                        this.confirmations = store.snapshot().confirmations;
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    }
                    event = user_events.recv() => {
                        let Ok(data) = event else { break };
                        if data.get("conversation_id").and_then(serde_json::Value::as_str) != Some(conversation_id.as_str()) {
                            continue;
                        }
                        let _ = cx.update(|cx| {
                            if let Some(entity) = weak_entity.upgrade() {
                                entity.update(cx, |this, cx| {
                                    if let Some(store) = AppState::global(cx).conversation_store() {
                                        let _ = store.apply_websocket_event("message.userCreated", &data);
                                        this.history_messages = store.message_snapshot().messages;
                                        this.confirmations = store.snapshot().confirmations;
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    }
                }
            }
        }).detach();
    }

    /// Subscribe to code selection events via EventHub
    pub fn subscribe_to_code_selections(entity: &Entity<Self>, cx: &mut App) {
        crate::core::event_bus::subscribe_entity_to_code_selections(
            entity,
            AppState::global(cx).event_hub().clone(),
            "ConversationPanel",
            |panel, selection, cx| {
                panel.code_selections.push(selection.into());
                cx.notify();
            },
            cx,
        );
    }

    /// Handle paste event and add images to pasted_images list
    /// Returns true if we handled the paste (had images), false otherwise
    fn handle_paste(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        log::info!("Handling paste in ConversationPanel");

        let mut handled = false;
        if let Some(clipboard_item) = cx.read_from_clipboard() {
            for entry in clipboard_item.entries().iter() {
                if let ClipboardEntry::Image(image) = entry {
                    log::info!("Processing pasted image: {:?}", image.format);
                    let image = image.clone();
                    handled = true;

                    cx.spawn_in(
                        window,
                        async move |this, cx| match crate::utils::clipboard::image_to_file(image).await {
                            Ok(image) => {
                                _ = cx.update(move |_window, cx| {
                                    let _ = this.update(cx, |this, cx| {
                                        this.pasted_images.push(image);
                                        cx.notify();
                                    });
                                });
                            }
                            Err(error) => {
                                log::error!("failed to materialize clipboard image: {error}");
                            }
                        },
                    )
                    .detach();
                }
            }
        }
        handled
    }

    /// Send a message to the current session
    /// Dispatches SendMessageToSession action to workspace for handling
    fn send_message(
        &mut self,
        text: String,
        images: Vec<ClipboardImage>,
        code_selections: Vec<AddCodeSelection>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let files = code_selections
            .into_iter()
            .map(|selection| selection.file_path.into())
            .chain(images.iter().map(|image| image.path.clone()))
            .collect();
        let weak_entity = cx.entity().downgrade();
        let conversation_id = self.session_id.clone();
        let draft_assistant_id = self.draft_assistant_id.clone();
        let draft_workspace = self.draft_workspace.clone();
        cx.spawn(async move |_, cx| {
            let conversation_id = match conversation_id {
                Some(id) => id,
                None => {
                    let Some(assistant_id) = draft_assistant_id else {
                        log::warn!("Draft conversation requires an agent before sending");
                        return;
                    };
                    match store.create(None, assistant_id, draft_workspace).await {
                        Ok(conversation) => conversation.id,
                        Err(error) => {
                            log::warn!("AionCore conversation creation failed: {error:?}");
                            return;
                        }
                    }
                }
            };
            let result = store.send_message(&conversation_id, text, files).await;
            for image in images {
                crate::utils::clipboard::remove_file(image.path).await;
            }

            match result {
                Ok(response) => {
                    let _ = cx.update(|cx| {
                        if let Some(entity) = weak_entity.upgrade() {
                            entity.update(cx, |this, cx| {
                                this.session_id = Some(conversation_id.clone());
                                this.active_turn_id = Some(response.turn_id);
                                if let Some(conversation) = store
                                    .snapshot()
                                    .conversations
                                    .iter()
                                    .find(|conversation| conversation.id == conversation_id)
                                {
                                    this.conversation_title = Some(normalized_tab_title(&conversation.name));
                                }
                                this.session_status = Self::runtime_status(Some(&response.runtime));
                                this.history_messages = store.message_snapshot().messages;
                                cx.notify();
                            });
                            Self::subscribe_to_stream_events(&entity, conversation_id.clone(), cx);
                        }
                    });
                }
                Err(error) => log::warn!("AionCore message send failed: {error:?}"),
            }
        })
        .detach();
    }

    fn choose_draft_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let entity = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, window| {
            let Some(workspace) = crate::utils::pick_folder("Select conversation workspace").await else {
                return;
            };
            let _ = window.update(|_, cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.draft_workspace = Some(workspace);
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn clear_draft_workspace(&mut self, cx: &mut Context<Self>) {
        self.draft_workspace = None;
        cx.notify();
    }

    fn render_draft_config(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let selected = self.draft_assistant_id.clone();
        let selected_agent = self
            .draft_assistants
            .iter()
            .find(|assistant| Some(assistant.id.as_str()) == selected.as_deref())
            .map(|assistant| assistant.name.clone())
            .unwrap_or_else(|| "Choose agent".to_owned());
        let workspace = self
            .draft_workspace
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Choose folder")
            .to_owned();
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new("draft-workspace")
                            .icon(IconName::Folder)
                            .label(workspace)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| this.choose_draft_workspace(window, cx))),
                    )
                    .when(self.draft_workspace.is_some(), |this| {
                        this.child(
                            Button::new("clear-draft-workspace")
                                .icon(IconName::Close)
                                .ghost()
                                .xsmall()
                                .tooltip("Clear workspace folder")
                                .on_click(cx.listener(|this, _, _, cx| this.clear_draft_workspace(cx))),
                        )
                    }),
            )
            .child(
                Button::new("draft-agent-menu")
                    .icon(IconName::Bot)
                    .label(selected_agent)
                    .ghost()
                    .xsmall()
                    .dropdown_menu({
                        let assistants = self.draft_assistants.clone();
                        let panel = cx.entity();
                        move |menu, window, _| {
                            assistants.iter().fold(menu, |menu, assistant| {
                                let assistant_id = assistant.id.clone();
                                menu.item(PopupMenuItem::new(assistant.name.clone()).on_click(window.listener_for(
                                    &panel,
                                    move |this, _, _, cx| {
                                        this.draft_assistant_id = Some(assistant_id.clone());
                                        cx.notify();
                                    },
                                )))
                            })
                        }
                    }),
            )
            .into_any_element()
    }

    fn render_session_config(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let panel = cx.entity();
        let options = self
            .config_options
            .iter()
            .filter(|option| option.option_type == "select" && !option.options.is_empty())
            .cloned()
            .collect::<Vec<_>>();
        if options.is_empty() {
            return div().into_any_element();
        }

        h_flex()
            .w_full()
            .gap_1()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(cx.theme().border)
            .children(options.into_iter().map(|option| {
                let option_id = option.id.clone();
                let label = option
                    .label
                    .clone()
                    .or(option.name.clone())
                    .unwrap_or_else(|| option.id.clone());
                let value = option.current_value.clone().unwrap_or_else(|| "Select".to_owned());
                let pending = self.pending_config_options.contains(&option.id);
                let options = option.options.clone();
                let panel = panel.clone();

                Button::new(("session-config-option", confirmation_element_id(&option_id)))
                    .label(if pending {
                        format!("{label}: {value} (next turn)")
                    } else {
                        format!("{label}: {value}")
                    })
                    .ghost()
                    .xsmall()
                    .dropdown_menu(move |menu, window, _| {
                        let option_id = option_id.clone();
                        options.iter().fold(menu, |menu, item| {
                            let value = item.value.clone();
                            let item_label = item
                                .label
                                .clone()
                                .or(item.name.clone())
                                .unwrap_or_else(|| value.clone());
                            let option_id = option_id.clone();
                            menu.item(PopupMenuItem::new(item_label).on_click(
                                window.listener_for(&panel, move |this, _, _, cx| {
                                    this.set_config_option(option_id.clone(), value.clone(), cx)
                                }),
                            ))
                        })
                    })
            }))
            .into_any_element()
    }

    fn send_cancel_message(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let (Some(conversation_id), Some(turn_id)) = (self.session_id.clone(), self.active_turn_id.clone()) else {
            return;
        };
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let weak_entity = cx.entity().downgrade();
        cx.spawn(
            async move |_, cx| match store.cancel_message(&conversation_id, &turn_id).await {
                Ok(response) => {
                    let _ = cx.update(|cx| {
                        if let Some(entity) = weak_entity.upgrade() {
                            entity.update(cx, |this, cx| {
                                this.session_status = Self::runtime_status(Some(&response.runtime));
                                cx.notify();
                            });
                        }
                    });
                }
                Err(error) => log::warn!("AionCore message cancel failed: {error:?}"),
            },
        )
        .detach();
    }

    /// Check if the input should be disabled based on session status
    /// Returns true if the session is closed, failed, or not resumable
    fn runtime_status(runtime: Option<&ConversationRuntimeSummary>) -> Option<SessionStatusInfo> {
        let runtime = runtime?;
        let status = match runtime.state.as_str() {
            "starting" => SessionStatus::Pending,
            "running" | "cancelling" => SessionStatus::InProgress,
            "idle" => SessionStatus::Idle,
            _ => SessionStatus::Failed,
        };
        Some(SessionStatusInfo {
            agent_name: "AionCore".to_owned(),
            status,
            last_active: Utc::now(),
            message_count: 0,
        })
    }

    fn render_confirmation(&self, confirmation: &Confirmation, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = cx.theme();
        let title = confirmation.title.clone().unwrap_or_else(|| "Confirmation".to_owned());
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return v_flex().child("AionCore connection unavailable").into_any_element();
        };
        let Some(conversation_id) = self.session_id.clone() else {
            return v_flex().child("Conversation unavailable").into_any_element();
        };
        let question = confirmation.questions.as_ref().and_then(parse_single_ask_question);
        let card = v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded(px(6.))
            .border_1()
            .border_color(theme.warning)
            .child(div().text_sm().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
            .child(div().text_sm().child(confirmation.description.clone()));

        if let Some(question) = question {
            let selected_labels = self.ask_selections.get(&confirmation.id).cloned().unwrap_or_default();
            let mut card = card.child(div().text_sm().child(question.text.clone()));
            for (option_index, label) in question.labels.iter().enumerate() {
                let label = label.clone();
                if question.multi_select {
                    let confirmation_id = confirmation.id.clone();
                    card = card.child(
                        gpui_component::checkbox::Checkbox::new((
                            "ask-select",
                            confirmation_element_id(&confirmation.id).wrapping_add(option_index as u64),
                        ))
                        .label(label.clone())
                        .checked(selected_labels.contains(&label))
                        .on_click(cx.listener(move |this, checked, _, cx| {
                            let selections = this.ask_selections.entry(confirmation_id.clone()).or_default();
                            if *checked {
                                if !selections.contains(&label) {
                                    selections.push(label.clone());
                                }
                            } else {
                                selections.retain(|selected| selected != &label);
                            }
                            cx.notify();
                        })),
                    );
                } else {
                    let request = ask_answer_request(question.text.clone(), vec![label.clone()]);
                    card = card.child(answer_button(
                        (
                            "ask-answer",
                            confirmation_element_id(&confirmation.id).wrapping_add(option_index as u64),
                        ),
                        label,
                        store.clone(),
                        conversation_id.clone(),
                        confirmation.call_id.clone(),
                        confirmation.id.clone(),
                        request,
                        false,
                    ));
                }
            }

            if question.multi_select {
                let request = ask_answer_request(question.text, selected_labels.clone());
                card = card.child(answer_button(
                    ("ask-submit", confirmation.id.len()),
                    "Submit",
                    store.clone(),
                    conversation_id.clone(),
                    confirmation.call_id.clone(),
                    confirmation.id.clone(),
                    request,
                    selected_labels.is_empty(),
                ));
            }

            card.child(decline_button(
                confirmation.id.clone(),
                store,
                conversation_id,
                confirmation.call_id.clone(),
            ))
            .into_any_element()
        } else {
            let mut card = card.child(div().text_xs().text_color(theme.muted_foreground).child(
                if confirmation.questions.is_some() {
                    "This question format is not supported by the current GUI; no answer was sent."
                } else {
                    "Choose a server-provided option."
                },
            ));
            for (option_index, option) in confirmation.options.iter().enumerate() {
                let data = option.value.clone();
                let store = store.clone();
                let conversation_id = conversation_id.clone();
                let confirmation = confirmation.clone();
                card = card.child(
                    Button::new(("confirmation", option_index))
                        .label(option.label.clone())
                        .on_click(move |_, _, cx| {
                            let store = store.clone();
                            let conversation_id = conversation_id.clone();
                            let confirmation = confirmation.clone();
                            let data = data.clone();
                            cx.spawn(async move |cx| {
                                let _ = store
                                    .respond_confirmation(&conversation_id, &confirmation, data, false)
                                    .await;
                                let _ = cx.update(|_| {});
                            })
                            .detach();
                        }),
                );
            }
            card.into_any_element()
        }
    }

    fn is_input_disabled(&self) -> bool {
        match &self.session_status {
            Some(status_info) => {
                matches!(status_info.status, SessionStatus::Closed | SessionStatus::Failed)
            }
            // If no session_id, allow input (new conversation mode)
            // If session_id exists but no status yet, allow input (status will be updated)
            None => false,
        }
    }

    /// Render the loading skeleton and status info when session is in progress
    fn render_loading_skeleton(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Only show loading skeleton when session is actively processing
        let should_show_loading = self.session_status.as_ref().map_or(false, |status_info| {
            matches!(status_info.status, SessionStatus::InProgress | SessionStatus::Pending)
        });

        if !should_show_loading {
            return v_flex().into_any_element();
        }

        // Build status indicator row
        let status_info = self.session_status.as_ref().unwrap(); // Safe because of check above
        let (status_icon, status_color) = match status_info.status {
            SessionStatus::InProgress => (IconName::Loader, cx.theme().primary),
            SessionStatus::Pending => (IconName::LoaderCircle, cx.theme().warning),
            _ => return v_flex().into_any_element(), // Fallback
        };

        // Calculate elapsed time from last_active
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(status_info.last_active);
        let total_seconds = duration.num_seconds().max(0) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let elapsed_time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

        // Main skeleton layout: horizontal layout with avatar spinner + status info + content skeletons
        v_flex()
            .w_full()
            .gap_3()
            .child(
                // Top row: Spinner avatar + status info (task + time) horizontally aligned
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        // Agent avatar as spinner with status icon
                        Spinner::new()
                            .icon(status_icon.clone())
                            .with_size(gpui_component::Size::Medium)
                            .color(status_color),
                    )
                    .child(
                        // Status info row: task + time
                        h_flex().items_center().gap_2p5().flex_1().child(
                            // Elapsed time display
                            h_flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    Icon::new(IconName::Info)
                                        .size(px(12.))
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(elapsed_time),
                                ),
                        ),
                    ),
            )
            .child(
                // Content skeletons - indented to align with text content
                h_flex()
                    .gap_3()
                    .child(
                        // Spacer to align with content (matches spinner width)
                        div().w(px(24.)),
                    )
                    .child(
                        // Message content skeletons - simulate text lines with varying widths
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .child(
                                Skeleton::new()
                                    .w_full()
                                    .max_w(px(480.))
                                    .h(px(16.))
                                    .rounded(cx.theme().radius),
                            )
                            .child(
                                Skeleton::new()
                                    .w_full()
                                    .max_w(px(420.))
                                    .h(px(16.))
                                    .rounded(cx.theme().radius),
                            )
                            .child(
                                Skeleton::new()
                                    .w_full()
                                    .max_w(px(360.))
                                    .h(px(16.))
                                    .rounded(cx.theme().radius),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn clear_confirmed_options(pending: &mut std::collections::HashSet<String>, options: &[AcpConfigOption]) {
    for option in options {
        if option.current_value.is_some() {
            pending.remove(&option.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AcpConfigOption, clear_confirmed_options, normalized_tab_title, workspace_from_extra};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn tab_title_is_single_line_and_truncated() {
        let title = normalized_tab_title("A\nconversation with a very long generated title");
        assert_eq!(title, "A conversation with a very long ...");
    }

    #[test]
    fn workspace_path_comes_from_conversation_extra() {
        let workspace = workspace_from_extra(&serde_json::json!({ "workspace": "/work/project" }));

        assert_eq!(workspace, Some(PathBuf::from("/work/project")));
    }

    #[test]
    fn workspace_path_ignores_missing_or_invalid_extra() {
        assert_eq!(workspace_from_extra(&serde_json::json!({})), None);
        assert_eq!(workspace_from_extra(&serde_json::json!({ "workspace": 42 })), None);
    }

    #[test]
    fn config_snapshot_clears_pending_option() {
        let options = vec![AcpConfigOption {
            id: "mode".to_owned(),
            name: None,
            label: None,
            description: None,
            category: None,
            option_type: "select".to_owned(),
            current_value: Some("plan".to_owned()),
            options: Vec::new(),
        }];
        let mut pending = ["mode".to_owned()].into_iter().collect();
        clear_confirmed_options(&mut pending, &options);
        assert!(!pending.contains("mode"));
    }

    #[test]
    fn mode_info_clears_pending_mode() {
        let mut options = vec![AcpConfigOption {
            id: "mode".to_owned(),
            name: None,
            label: None,
            description: None,
            category: None,
            option_type: "select".to_owned(),
            current_value: Some("plan".to_owned()),
            options: Vec::new(),
        }];
        let mut pending = ["mode".to_owned()].into_iter().collect();
        options[0].current_value = Some("plan".to_owned());
        clear_confirmed_options(&mut pending, &options);
        assert_eq!(options[0].current_value.as_deref(), Some("plan"));
        assert!(!pending.contains("mode"));
    }
}

impl DockPanel for ConversationPanel {
    fn title() -> &'static str {
        "Conversation"
    }

    fn title_key() -> Option<&'static str> {
        Some("conversation.title")
    }

    fn description() -> &'static str {
        "Conversation panel backed by AionCore"
    }

    fn closable() -> bool {
        true
    }

    fn zoomable() -> Option<gpui_component::dock::PanelControl> {
        Some(gpui_component::dock::PanelControl::default())
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn on_active_any(view: gpui::AnyView, active: bool, window: &mut Window, cx: &mut App) {
        let _ = window;
        let Ok(conversation) = view.downcast::<Self>() else {
            return;
        };
        conversation.update(cx, |conversation, cx| {
            conversation.is_active = active;
            conversation.update_active_workspace(cx);
        });
    }

    fn paddings() -> gpui::Pixels {
        px(0.)
    }
}

impl Focusable for ConversationPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ConversationPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_empty = self.history_messages.is_empty() && self.confirmations.is_empty();
        let message_list = v_flex()
            .p_4()
            .gap_3()
            .bg(cx.theme().background)
            .children(
                self.history_messages
                    .iter()
                    .filter(|message| !message.hidden)
                    .map(|message| self.render_history_message(message, window, cx)),
            )
            .when_some(self.prompt_capability, |this, capability| {
                this.when(!capability.0 && !capability.1, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("This agent has no native media prompt capability; attachments are sent as files."),
                    )
                })
            })
            .when(self.prompt_capability.is_none(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Media capability is unavailable; attachments are sent as files."),
                )
            })
            .children(
                self.confirmations
                    .iter()
                    .map(|confirmation| self.render_confirmation(confirmation, cx)),
            )
            .child(self.render_loading_skeleton(cx));

        // Main layout: vertical flex with scroll area on top and input box at bottom
        v_flex()
            .id("messages")
            .size_full()
            .child(
                // Scrollable message area - takes remaining space
                div()
                    .id("conversation-scroll-container")
                    .flex_1()
                    .w_full()
                    .track_scroll(&self.scroll_handle)
                    .overflow_y_scroll()
                    .size_full()
                    .when(is_empty, |this| {
                        // Show empty state with centered text
                        this.child(
                            div().size_full().flex().items_center().justify_center().child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .text_sm()
                                    .child(t!("conversation.empty").to_string()),
                            ),
                        )
                    })
                    .when(!is_empty, |this| {
                        // Show message list
                        this.pb_3() // Add padding at bottom so messages don't get hidden behind input box
                            .child(message_list)
                    }),
            )
            .child(
                // Chat input box at bottom (fixed, not scrollable)
                div()
                    .flex_none() // Don't allow shrinking
                    .w_full()
                    .bg(cx.theme().background) // Solid background
                    // .border_t_1()
                    .p_1()
                    .when(self.session_id.is_none(), |this| {
                        this.child(self.render_draft_config(cx))
                    })
                    .when(self.session_id.is_some(), |this| {
                        this.child(self.render_session_config(cx))
                    })
                    // .border_color(cx.theme().border)
                    .child({
                        let entity = cx.entity().clone();
                        let is_disabled = self.is_input_disabled();
                        let command_suggestions = self
                            .slash_commands
                            .iter()
                            .map(|command| AvailableCommand::new(command.command.clone(), command.description.clone()))
                            .collect::<Vec<_>>();
                        let show_command_suggestions = self.input_state.read(cx).value().trim_start().starts_with('/');
                        ChatInputBox::new("chat-input", self.input_state.clone())
                            .pasted_images(self.pasted_images.clone())
                            .code_selections(self.code_selections.clone())
                            .session_status(self.session_status.as_ref().map(|info| info.status.clone()))
                            .agent_icon(self.agent_icon.clone().unwrap_or(IconName::Bot))
                            .when_some(self.agent_name.clone(), |this, name| this.agent_name(name))
                            .command_suggestions(command_suggestions)
                            .show_command_suggestions(show_command_suggestions)
                            .disabled(is_disabled)
                            .on_paste(move |window, cx| {
                                entity.update(cx, |this, cx| {
                                    this.handle_paste(window, cx);
                                });
                            })
                            .on_remove_image(cx.listener(|this, idx, _, cx| {
                                // Remove the image at the given index
                                if *idx < this.pasted_images.len() {
                                    let image = this.pasted_images.remove(*idx);
                                    crate::utils::clipboard::remove_file_sync(&image.path);
                                    cx.notify();
                                }
                            }))
                            .on_remove_code_selection(cx.listener(|this, idx, _, cx| {
                                // Remove the code selection at the given index
                                if *idx < this.code_selections.len() {
                                    this.code_selections.remove(*idx);
                                    cx.notify();
                                }
                            }))
                            .on_send(cx.listener(|this, _ev, window, cx| {
                                let text = this.input_state.read(cx).value().to_string();
                                if !text.trim().is_empty()
                                    || !this.pasted_images.is_empty()
                                    || !this.code_selections.is_empty()
                                {
                                    // Clear the input
                                    this.input_state.update(cx, |state, cx| {
                                        state.set_value(SharedString::from(""), window, cx);
                                    });

                                    // Send the message with images and code selections
                                    let images = std::mem::take(&mut this.pasted_images);
                                    let code_selections = std::mem::take(&mut this.code_selections);
                                    this.send_message(text, images, code_selections, window, cx);

                                    cx.notify();
                                }
                            }))
                            .on_cancel(cx.listener(|this, _ev, window, cx| {
                                log::info!("[ConversationPanel] on_cancel callback triggered");
                                this.send_cancel_message(window, cx);
                                cx.notify();
                            }))
                    }),
            )
    }
}
