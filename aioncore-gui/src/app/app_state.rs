use gpui::{App, AppContext, Entity, Global, SharedString};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{
    core::aioncore::{
        ConnectionHandle, ConnectionStatus, ConversationStore, CoreClient, ProjectStore, SettingsStore, TeamStore,
    },
    core::config::DEFAULT_TOOL_CALL_PREVIEW_MAX_LINES,
    core::event_bus::EventHub,
};

use super::service_registry::ServiceRegistry;

/// Welcome session info - stores the session created when user selects an agent
#[derive(Clone, Debug)]
pub struct WelcomeSession {
    pub session_id: String,
    pub agent_name: String,
}

pub struct AppState {
    // UI state (GPUI entities)
    pub invisible_panels: Entity<Vec<SharedString>>,
    pub selected_tool_call: Entity<Option<agent_client_protocol::schema::ToolCall>>,

    // Infrastructure
    core_client: Option<Arc<CoreClient>>,
    conversation_store: Option<ConversationStore>,
    settings_store: Option<SettingsStore>,
    project_store: Option<ProjectStore>,
    team_store: Option<TeamStore>,
    core_connection: Option<ConnectionHandle>,

    /// Service registry — Clone + Send, can be captured in async closures
    pub services: ServiceRegistry,

    // Configuration
    current_working_dir: PathBuf,
    tool_call_preview_max_lines: usize,

    // Temporary UI state
    welcome_session: Option<WelcomeSession>,
    app_title: SharedString,
}

impl AppState {
    pub fn init(cx: &mut App) {
        let services = ServiceRegistry::new(EventHub::new());

        let state = Self {
            invisible_panels: cx.new(|_| Vec::new()),
            core_client: None,
            conversation_store: None,
            settings_store: None,
            project_store: None,
            team_store: None,
            core_connection: None,
            services,
            welcome_session: None,
            current_working_dir: Self::resolve_initial_working_dir(),
            tool_call_preview_max_lines: DEFAULT_TOOL_CALL_PREVIEW_MAX_LINES,
            selected_tool_call: cx.new(|_| None),
            app_title: SharedString::from(""),
        };
        cx.set_global::<AppState>(state);
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub fn connect_core(&mut self, backend_url: &str) -> Result<(), crate::core::aioncore::CoreClientError> {
        let client = Arc::new(CoreClient::new(backend_url)?);
        let connection = client.connect();
        self.conversation_store = Some(ConversationStore::new(client.clone()));
        self.settings_store = Some(SettingsStore::new(client.clone()));
        self.project_store = Some(ProjectStore::new(client.clone()));
        self.team_store = Some(TeamStore::new(client.clone()));
        self.core_client = Some(client);
        self.core_connection = Some(connection);
        Ok(())
    }

    pub fn core_connection_status(&self) -> ConnectionStatus {
        self.core_client
            .as_ref()
            .map(|client| client.connection_status())
            .unwrap_or(ConnectionStatus::Disconnected)
    }

    pub fn core_client(&self) -> Option<&Arc<CoreClient>> {
        self.core_client.as_ref()
    }

    pub fn conversation_store(&self) -> Option<&ConversationStore> {
        self.conversation_store.as_ref()
    }

    pub fn settings_store(&self) -> Option<&SettingsStore> {
        self.settings_store.as_ref()
    }

    pub fn project_store(&self) -> Option<&ProjectStore> {
        self.project_store.as_ref()
    }

    pub fn team_store(&self) -> Option<&TeamStore> {
        self.team_store.as_ref()
    }

    fn resolve_initial_working_dir() -> PathBuf {
        if let Ok(cwd) = std::env::current_dir() {
            if Self::is_safe_working_dir(&cwd) {
                return cwd;
            }
        }

        let data_dir = crate::core::config_manager::user_data_dir_or_temp();
        if Self::is_safe_working_dir(&data_dir) {
            return data_dir;
        }

        if let Some(home) = dirs::home_dir() {
            if Self::is_safe_working_dir(&home) {
                return home;
            }
        }

        data_dir
    }

    fn is_safe_working_dir(path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }

        if path.parent().is_none() {
            return false;
        }

        #[cfg(target_os = "macos")]
        {
            let path_str = path.to_string_lossy();
            if path_str.contains(".app/Contents") {
                return false;
            }
        }

        true
    }

    pub fn set_app_title(&mut self, title: SharedString) {
        self.app_title = title;
    }

    pub fn app_title(&self) -> &SharedString {
        &self.app_title
    }

    /// Get the event hub
    pub fn event_hub(&self) -> &EventHub {
        &self.services.event_hub
    }

    /// Get the current working directory
    pub fn current_working_dir(&self) -> &PathBuf {
        &self.current_working_dir
    }

    /// Set the current working directory
    pub fn set_current_working_dir(&mut self, path: PathBuf) {
        log::info!("Setting current working directory: {:?}", path);
        self.current_working_dir = path;
    }

    /// Get the tool call preview line limit
    pub fn tool_call_preview_max_lines(&self) -> usize {
        self.tool_call_preview_max_lines
    }
}
impl Global for AppState {}
