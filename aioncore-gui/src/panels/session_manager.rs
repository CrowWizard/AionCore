use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render, Styled, Window,
    prelude::FluentBuilder, px,
};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

use crate::{
    AppState, PanelAction,
    core::aioncore::{ConversationEventAction, ConversationState, ConversationStore},
    panels::dock_panel::DockPanel,
};

pub struct SessionManagerPanel {
    focus_handle: FocusHandle,
    state: ConversationState,
}

impl DockPanel for SessionManagerPanel {
    fn title() -> &'static str {
        "Conversations"
    }

    fn description() -> &'static str {
        "AionCore conversations"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn paddings() -> gpui::Pixels {
        px(12.)
    }
}

impl SessionManagerPanel {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            state: ConversationState::default(),
        };
        panel.refresh(cx);
        panel.subscribe_events(cx);
        panel
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        self.state.is_loading = true;
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = store.refresh_list().await;
            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.state = store.snapshot();
                        if result.is_err() {
                            this.state.error = Some("Unable to load conversations".to_owned());
                        }
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn create_conversation(&mut self, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = store.create(None).await;
            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.state = store.snapshot();
                        if result.is_err() {
                            this.state.error = Some("Unable to create conversation".to_owned());
                        }
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn delete_conversation(&mut self, conversation_id: String, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = store.delete(&conversation_id).await;
            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.state = store.snapshot();
                        if result.is_err() {
                            this.state.error = Some("Unable to delete conversation".to_owned());
                        }
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn open_conversation(&mut self, conversation_id: String, window: &mut Window, cx: &mut Context<Self>) {
        window.dispatch_action(
            Box::new(PanelAction::add_conversation_for_session(
                conversation_id,
                gpui_component::dock::DockPlacement::Center,
            )),
            cx,
        );
    }

    fn subscribe_events(&mut self, cx: &mut Context<Self>) {
        let Some(client) = AppState::global(cx).core_client().cloned() else {
            return;
        };
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let entity = cx.entity().downgrade();
        let mut list_changed = client.events().subscribe_websocket_event("conversation.listChanged");
        let mut name_updated = client.events().subscribe_websocket_event("conversation.nameUpdated");
        cx.spawn(async move |_, cx| {
            loop {
                let event = tokio::select! {
                    value = list_changed.recv() => value.map(|data| ("conversation.listChanged", data)),
                    value = name_updated.recv() => value.map(|data| ("conversation.nameUpdated", data)),
                };
                let Ok((name, data)) = event else {
                    break;
                };
                let action = store.apply_websocket_event(name, &data);
                match action {
                    ConversationEventAction::RefreshList => {
                        let _ = store.refresh_list().await;
                    }
                    ConversationEventAction::RefreshSelected => {
                        let state = store.snapshot();
                        if let Some(conversation_id) = state.selected_conversation_id {
                            let _ = store.load_conversation(&conversation_id).await;
                        } else {
                            let _ = store.refresh_list().await;
                        }
                    }
                    ConversationEventAction::Ignored => continue,
                }
                let _ = cx.update(|cx| {
                    if let Some(entity) = entity.upgrade() {
                        entity.update(cx, |this, cx| {
                            this.state = store.snapshot();
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }
}

impl Focusable for SessionManagerPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SessionManagerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .gap_2()
            .bg(theme.background)
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        gpui::div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Conversations"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("new-conversation")
                                    .icon(Icon::new(IconName::Plus))
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| this.create_conversation(cx))),
                            )
                            .child(
                                Button::new("refresh-conversations")
                                    .icon(Icon::new(IconName::LoaderCircle))
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                            ),
                    ),
            )
            .when_some(self.state.error.clone(), |this, error| {
                this.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.colors.danger_foreground)
                        .child(error),
                )
            })
            .child(
                gpui::div().flex_1().min_h_0().overflow_y_scrollbar().child(
                    v_flex()
                        .gap_1()
                        .children(
                            self.state
                                .conversations
                                .iter()
                                .enumerate()
                                .map(|(index, conversation)| {
                                    let open_id = conversation.id.clone();
                                    let delete_id = conversation.id.clone();
                                    h_flex()
                                        .w_full()
                                        .items_center()
                                        .gap_1()
                                        .p_2()
                                        .rounded(px(6.))
                                        .bg(theme.secondary)
                                        .child(
                                            Button::new(("open-conversation", index))
                                                .label(conversation.name.clone())
                                                .ghost()
                                                .flex_1()
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.open_conversation(open_id.clone(), window, cx)
                                                })),
                                        )
                                        .child(
                                            Button::new(("delete-conversation", index))
                                                .icon(Icon::new(IconName::Delete))
                                                .ghost()
                                                .small()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.delete_conversation(delete_id.clone(), cx)
                                                })),
                                        )
                                }),
                        ),
                ),
            )
    }
}
