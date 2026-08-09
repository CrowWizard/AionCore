// Panel-related modules

#[path = "settings_panel/types.rs"]
mod app_settings;
pub mod conversation;
pub mod dock_panel;
mod project_panel;
mod session_manager;
mod team_panel;
mod terminal_panel;

pub use app_settings::AppSettings;
pub use conversation::ConversationPanel;
pub use dock_panel::{DockPanel, DockPanelContainer, DockPanelState};
pub use project_panel::ProjectPanel;
pub use session_manager::SessionManagerPanel;
pub use team_panel::TeamPanel;
pub use terminal_panel::TerminalPanel;
