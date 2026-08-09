use gpui::*;
use gpui_component::dock::{DockItem, DockPlacement};

use crate::{
    ConversationPanel, CreateTaskFromWelcome, NewSessionConversationPanel, SendMessageToSession,
    app::actions::CancelSession,
    panels::{DockPanel, dock_panel::DockPanelContainer},
};

use crate::workspace::DockWorkspace;

impl DockWorkspace {
    pub(in crate::workspace) fn on_action_new_session_conversation_panel(
        &mut self,
        _action: &NewSessionConversationPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_conversation_panel(window, cx);
    }

    pub(in crate::workspace) fn on_action_create_task_from_welcome(
        &mut self,
        _action: &CreateTaskFromWelcome,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        log::warn!("Local agent task creation is unavailable in AionCore GUI");
    }

    pub fn panel_for_session(session_id: String, window: &mut Window, cx: &mut App) -> Entity<DockPanelContainer> {
        let name = ConversationPanel::title();
        let description = ConversationPanel::description();
        let agent_studio = ConversationPanel::view_for_session(session_id, window, cx);
        let agent_studio_klass = ConversationPanel::klass();

        cx.new(|cx| {
            let mut agent_studio = DockPanelContainer::new(cx)
                .agent_studio(agent_studio.into(), agent_studio_klass)
                .on_active(ConversationPanel::on_active_any);
            agent_studio.focus_handle = cx.focus_handle();
            agent_studio.closable = ConversationPanel::closable();
            agent_studio.zoomable = ConversationPanel::zoomable();
            agent_studio.name = name.into();
            agent_studio.description = description.into();
            agent_studio.title_bg = ConversationPanel::title_bg();
            agent_studio.paddings = ConversationPanel::paddings();
            agent_studio
        })
    }

    pub(in crate::workspace) fn on_action_send_message_to_session(
        &mut self,
        _action: &SendMessageToSession,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        log::warn!("Legacy session message dispatch is unavailable in AionCore GUI");
    }

    pub(in crate::workspace) fn on_action_cancel_session(
        &mut self,
        _action: &CancelSession,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        log::warn!("Legacy session cancellation is unavailable in AionCore GUI");
    }
}
