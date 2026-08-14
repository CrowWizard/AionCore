use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
    v_flex,
};
use std::{collections::BTreeMap, path::Path};

use crate::{
    AppState, PanelAction,
    core::aioncore::{
        AionrsSessionResponse, AssistantResponse, ConversationEventAction, ConversationState, ConversationStore,
        models::ConversationResponse,
    },
    panels::dock_panel::DockPanel,
    utils,
};

pub struct SessionManagerPanel {
    focus_handle: FocusHandle,
    tree_state: Entity<TreeState>,
    tree_items: Vec<TreeItem>,
    state: ConversationState,
    available_assistants: Vec<AssistantResponse>,
    loading_assistants: bool,
    aionrs_sessions: Vec<AionrsSessionResponse>,
    loading_aionrs_sessions: bool,
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
        let tree_state = cx.new(|cx| TreeState::new(cx));
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            tree_state,
            tree_items: Vec::new(),
            state: ConversationState::default(),
            available_assistants: Vec::new(),
            loading_assistants: false,
            aionrs_sessions: Vec::new(),
            loading_aionrs_sessions: false,
        };
        panel.refresh(cx);
        panel.refresh_assistants(cx);
        panel.refresh_aionrs_sessions(cx);
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
                        this.sync_tree(cx);
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

    fn refresh_assistants(&mut self, cx: &mut Context<Self>) {
        let Some(settings_store) = AppState::global(cx).settings_store().cloned() else {
            return;
        };
        self.loading_assistants = true;
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = settings_store.refresh_assistants().await;
            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.loading_assistants = false;
                        match result {
                            Ok(assistants) => {
                                this.available_assistants =
                                    assistants.into_iter().filter(|assistant| assistant.enabled).collect();
                            }
                            Err(_) => this.state.error = Some("Unable to load assistants".to_owned()),
                        }
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn refresh_aionrs_sessions(&mut self, cx: &mut Context<Self>) {
        let Some(settings_store) = AppState::global(cx).settings_store().cloned() else {
            return;
        };
        self.loading_aionrs_sessions = true;
        let entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = settings_store.list_aionrs_sessions().await;
            let _ = cx.update(|cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.loading_aionrs_sessions = false;
                        match result {
                            Ok(sessions) => this.aionrs_sessions = sessions,
                            Err(_) => this.state.error = Some("Unable to load persisted aionrs sessions".to_owned()),
                        }
                        this.sync_tree(cx);
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn create_conversation(&mut self, assistant: AssistantResponse, window: &mut Window, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).conversation_store().cloned() else {
            return;
        };
        let entity = cx.entity().downgrade();
        cx.spawn_in(window, async move |_, window| {
            let Some(workspace) = utils::pick_folder("Select conversation workspace").await else {
                return;
            };
            let result = store.create(None, assistant.id, workspace).await;
            let _ = window.update(|_, cx| {
                if let Some(entity) = entity.upgrade() {
                    entity.update(cx, |this, cx| {
                        this.state = store.snapshot();
                        this.sync_tree(cx);
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
                        this.sync_tree(cx);
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
                            this.sync_tree(cx);
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }

    fn sync_tree(&mut self, cx: &mut Context<Self>) {
        let selected_id = self.tree_state.read(cx).selected_item().map(|item| item.id.clone());
        let expanded = expanded_tree_items(&self.tree_items);
        let items = build_conversation_tree(&self.state, &self.aionrs_sessions, &expanded);
        let selected_item = selected_id
            .as_ref()
            .and_then(|id| find_tree_item(&items, id.as_str()))
            .cloned();
        self.tree_items = items.clone();
        self.tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
            state.set_selected_item(selected_item.as_ref(), cx);
        });
    }

    fn delete_selected_conversation(&mut self, cx: &mut Context<Self>) {
        let selected_id = self.selected_conversation_id(cx);
        if let Some(conversation_id) = selected_id {
            self.delete_conversation(conversation_id, cx);
        }
    }

    fn selected_conversation_id(&self, cx: &App) -> Option<String> {
        self.tree_state
            .read(cx)
            .selected_item()
            .and_then(|item| item.id.as_str().strip_prefix("conversation:"))
            .map(str::to_owned)
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = cx.entity();
        let tree_state = self.tree_state.clone();
        tree(&self.tree_state, move |ix, entry, _selected, _window, cx| {
            let item = entry.item().clone();
            let is_conversation = item.id.as_str().starts_with("conversation:");
            let is_saved_session = item.id.as_str().starts_with("aionrs:");
            let icon = if entry.is_folder() {
                if entry.is_expanded() {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                }
            } else if is_saved_session {
                IconName::Bot
            } else {
                IconName::File
            };

            ListItem::new(ix)
                .w_full()
                .rounded(cx.theme().radius)
                .py_1()
                .px_2()
                .pl(px(14.) * entry.depth() + px(8.))
                .child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_2()
                        .child(Icon::new(icon).size(px(14.)).text_color(cx.theme().muted_foreground))
                        .child(gpui::div().flex_1().min_w_0().text_ellipsis().child(item.label.clone())),
                )
                .on_click({
                    let panel = panel.clone();
                    let tree_state = tree_state.clone();
                    move |_, window, cx| {
                        tree_state.update(cx, |state, cx| state.focus(window, cx));
                        if !is_conversation {
                            return;
                        }
                        let Some(conversation_id) = item.id.as_str().strip_prefix("conversation:") else {
                            return;
                        };
                        panel.update(cx, |this, cx| {
                            this.open_conversation(conversation_id.to_owned(), window, cx);
                        });
                    }
                })
        })
        .text_sm()
        .bg(cx.theme().sidebar)
        .text_color(cx.theme().sidebar_foreground)
        .size_full()
    }
}

fn build_conversation_tree(
    state: &ConversationState,
    aionrs_sessions: &[AionrsSessionResponse],
    expanded: &BTreeMap<String, bool>,
) -> Vec<TreeItem> {
    let mut by_workspace = BTreeMap::<String, Vec<TreeItem>>::new();
    for conversation in &state.conversations {
        let workspace = conversation
            .extra
            .get("workspace")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("Other");
        by_workspace
            .entry(workspace.to_owned())
            .or_default()
            .push(TreeItem::new(
                format!("conversation:{}", conversation.id),
                conversation.name.clone(),
            ));
    }

    let mut items = by_workspace
        .into_iter()
        .map(|(workspace, conversations)| {
            let label = workspace_label(&workspace);
            TreeItem::new(format!("workspace:{workspace}"), label)
                .expanded(expanded.get(&format!("workspace:{workspace}")).copied().unwrap_or(true))
                .children(conversations)
        })
        .collect::<Vec<_>>();

    if !aionrs_sessions.is_empty() {
        items.push(
            TreeItem::new("saved-aionrs", "Saved aionrs Sessions")
                .expanded(expanded.get("saved-aionrs").copied().unwrap_or(false))
                .children(aionrs_sessions.iter().map(|session| {
                    TreeItem::new(
                        format!("aionrs:{}", session.id),
                        format!(
                            "{} · {} · {} messages",
                            session.summary, session.model, session.message_count
                        ),
                    )
                    .disabled(true)
                })),
        );
    }
    items
}

fn workspace_label(workspace: &str) -> String {
    let path = Path::new(workspace);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(workspace);
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty());
    parent
        .map(|parent| format!("{name} · {parent}"))
        .unwrap_or_else(|| name.to_owned())
}

fn expanded_tree_items(items: &[TreeItem]) -> BTreeMap<String, bool> {
    fn collect(item: &TreeItem, expanded: &mut BTreeMap<String, bool>) {
        if item.is_folder() {
            expanded.insert(item.id.to_string(), item.is_expanded());
        }
        for child in &item.children {
            collect(child, expanded);
        }
    }

    let mut expanded = BTreeMap::new();
    for item in items {
        collect(item, &mut expanded);
    }
    expanded
}

fn find_tree_item<'a>(items: &'a [TreeItem], id: &str) -> Option<&'a TreeItem> {
    for item in items {
        if item.id.as_str() == id {
            return Some(item);
        }
        if let Some(found) = find_tree_item(&item.children, id) {
            return Some(found);
        }
    }
    None
}

impl Focusable for SessionManagerPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SessionManagerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let can_delete = self.selected_conversation_id(cx).is_some();
        let tree_is_empty = self.state.conversations.is_empty() && self.aionrs_sessions.is_empty();
        let tree_is_loading = self.state.is_loading || self.loading_aionrs_sessions;
        v_flex()
            .size_full()
            .bg(theme.background)
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        gpui::div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Conversations"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("delete-selected-conversation")
                                    .icon(Icon::new(IconName::Delete))
                                    .ghost()
                                    .xsmall()
                                    .disabled(!can_delete)
                                    .tooltip("Delete selected conversation")
                                    .on_click(cx.listener(|this, _, _, cx| this.delete_selected_conversation(cx))),
                            )
                            .child(
                                Button::new("refresh-conversations")
                                    .icon(Icon::new(IconName::LoaderCircle))
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Refresh conversations")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.refresh(cx);
                                        this.refresh_aionrs_sessions(cx);
                                    })),
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
                v_flex()
                    .px_2()
                    .py_2()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(if self.loading_assistants {
                                "Loading assistants..."
                            } else {
                                "New conversation"
                            }),
                    )
                    .children(self.available_assistants.iter().enumerate().map(|(index, assistant)| {
                        let assistant = assistant.clone();
                        Button::new(("new-conversation", index))
                            .label(assistant.name.clone())
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.create_conversation(assistant.clone(), window, cx)
                            }))
                    })),
            )
            .child(
                gpui::div()
                    .flex_1()
                    .min_h_0()
                    .when(tree_is_empty, |this| {
                        this.child(
                            v_flex()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .gap_1()
                                .px_4()
                                .text_center()
                                .child(gpui::div().text_sm().font_weight(gpui::FontWeight::MEDIUM).child(
                                    if tree_is_loading {
                                        "Loading conversations..."
                                    } else {
                                        "No conversations"
                                    },
                                ))
                                .when(!tree_is_loading, |this| {
                                    this.child(
                                        gpui::div()
                                            .text_xs()
                                            .text_color(theme.muted_foreground)
                                            .child("Choose an assistant above to start in a workspace."),
                                    )
                                }),
                        )
                    })
                    .when(!tree_is_empty, |this| this.child(self.render_tree(cx))),
            )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    fn conversation(id: &str, name: &str, workspace: &str) -> ConversationResponse {
        ConversationResponse {
            id: id.to_owned(),
            name: name.to_owned(),
            pinned: false,
            status: json!("idle"),
            runtime: None,
            r#type: json!("acp"),
            extra: json!({ "workspace": workspace }),
            prompt_capability: None,
        }
    }

    #[test]
    fn groups_conversations_by_workspace_and_keeps_saved_sessions_separate() {
        let state = ConversationState {
            conversations: vec![
                conversation("c1", "First", "/work/alpha"),
                conversation("c2", "Second", "/work/alpha"),
                conversation("c3", "Third", "/work/beta"),
            ],
            ..Default::default()
        };
        let saved = vec![AionrsSessionResponse {
            id: "saved-1".into(),
            created_at: "2026-08-15".into(),
            updated_at: "2026-08-15".into(),
            model: "model".into(),
            summary: "Saved work".into(),
            message_count: 2,
        }];

        let items = build_conversation_tree(&state, &saved, &BTreeMap::new());

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].label.as_str(), "alpha · work");
        assert_eq!(items[0].children.len(), 2);
        assert_eq!(items[1].label.as_str(), "beta · work");
        assert_eq!(items[1].children.len(), 1);
        assert_eq!(items[2].id.as_str(), "saved-aionrs");
        assert_eq!(items[2].children[0].id.as_str(), "aionrs:saved-1");
        assert!(items[2].children[0].is_disabled());
    }

    #[test]
    fn preserves_expansion_state_when_tree_data_refreshes() {
        let state = ConversationState {
            conversations: vec![conversation("c1", "First", "/work/alpha")],
            ..Default::default()
        };
        let initial = build_conversation_tree(&state, &[], &BTreeMap::new());
        let collapsed = initial[0].clone().expanded(false);
        let expanded = expanded_tree_items(&[collapsed]);

        let refreshed = build_conversation_tree(&state, &[], &expanded);

        assert!(!refreshed[0].is_expanded());
    }
}
