use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _, Render, Styled, Window,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::{
    AppState,
    core::aioncore::{TeamEventAction, TeamState},
    panels::dock_panel::DockPanel,
};

const TEAM_EVENTS: [&str; 9] = [
    "team.agentStatusChanged",
    "team.agentSpawned",
    "team.agentRemoved",
    "team.agentRenamed",
    "team.agentRuntimeStatusChanged",
    "team.sessionStatusChanged",
    "team.teammateMessage",
    "team.taskChanged",
    "team.mailboxChanged",
];

pub struct TeamPanel {
    focus_handle: FocusHandle,
    state: TeamState,
}

impl DockPanel for TeamPanel {
    fn title() -> &'static str {
        "Teams"
    }

    fn description() -> &'static str {
        "AionCore team list and runtime state"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn paddings() -> gpui::Pixels {
        px(12.)
    }
}

impl TeamPanel {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            state: TeamState::default(),
        };
        panel.refresh(cx);
        panel.subscribe_events(cx);
        panel
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).team_store().cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let _ = store.refresh_list().await;
            let snapshot = store.snapshot();
            let _ = cx.update(|cx| {
                if let Some(view) = view.upgrade() {
                    view.update(cx, |this, cx| {
                        this.state = snapshot;
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn select(&mut self, team_id: String, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).team_store().cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let _ = store.select(&team_id).await;
            let snapshot = store.snapshot();
            let _ = cx.update(|cx| {
                if let Some(view) = view.upgrade() {
                    view.update(cx, |this, cx| {
                        this.state = snapshot;
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn subscribe_events(&mut self, cx: &mut Context<Self>) {
        let Some(client) = AppState::global(cx).core_client().cloned() else {
            return;
        };
        let Some(store) = AppState::global(cx).team_store().cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        let mut receivers = TEAM_EVENTS
            .iter()
            .map(|name| client.events().subscribe_websocket_event(name))
            .collect::<Vec<_>>();
        cx.spawn(async move |_, cx| {
            loop {
                let mut received = None;
                for (index, receiver) in receivers.iter_mut().enumerate() {
                    match receiver.try_recv() {
                        Ok(data) => {
                            received = Some((TEAM_EVENTS[index], data));
                            break;
                        }
                        Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
                        Err(tokio::sync::broadcast::error::TryRecvError::Closed) => return,
                        Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                            received = Some((TEAM_EVENTS[index], serde_json::Value::Null));
                            break;
                        }
                    }
                }
                let Some((name, data)) = received else {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                };
                match store.apply_websocket_event(name, &data) {
                    TeamEventAction::RefreshList => {
                        let _ = store.refresh_list().await;
                    }
                    TeamEventAction::RefreshSelected => {
                        if let Some(team_id) = store.snapshot().selected_team_id {
                            let _ = store.select(&team_id).await;
                        } else {
                            let _ = store.refresh_list().await;
                        }
                    }
                    TeamEventAction::Ignored => {}
                }
                let snapshot = store.snapshot();
                let _ = cx.update(|cx| {
                    if let Some(view) = view.upgrade() {
                        view.update(cx, |this, cx| {
                            this.state = snapshot;
                            cx.notify();
                        });
                    }
                });
            }
        })
        .detach();
    }
}

impl Focusable for TeamPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TeamPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let mut content = v_flex().size_full().gap_2().bg(theme.background).child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(gpui::div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Teams"))
                .child(
                    Button::new("refresh-teams")
                        .icon(Icon::new(IconName::LoaderCircle))
                        .ghost()
                        .small()
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                ),
        );
        if let Some(error) = &self.state.error {
            content = content.child(gpui::div().text_sm().text_color(theme.danger).child(error.clone()));
        }
        for (index, team) in self.state.teams.iter().enumerate() {
            let team_id = team.id.clone();
            let selected = self.state.selected_team_id.as_deref() == Some(team.id.as_str());
            content = content.child(
                Button::new(("team", index))
                    .label(format!("{} · {} agents", team.name, team.assistants.len()))
                    .when(selected, |button| button.primary())
                    .when(!selected, |button| button.outline())
                    .small()
                    .on_click(cx.listener(move |this, _, _, cx| this.select(team_id.clone(), cx))),
            );
        }
        if self.state.teams.is_empty() && !self.state.is_loading {
            content = content.child(
                gpui::div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No teams returned by AionCore"),
            );
        }
        if let Some(team) = &self.state.selected_team {
            content = content.child(
                gpui::div()
                    .mt_3()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(team.name.clone()),
            );
            for agent in &team.assistants {
                let status = agent.status.as_deref().unwrap_or("unknown");
                content = content.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} · {} · {}", agent.name, agent.role, status)),
                );
            }
            content = content.child(
                gpui::div()
                    .mt_3()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Tasks ({})", self.state.tasks.len())),
            );
            if self.state.tasks.is_empty() {
                content = content.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("No tasks"),
                );
            }
            for task in self.state.tasks.iter().take(50) {
                let owner = task.owner.as_deref().unwrap_or("unassigned");
                let description = task.description.as_deref().unwrap_or("No description");
                content = content.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} · {} · {}", task.subject, task.status, owner))
                        .child(gpui::div().text_xs().child(description.to_owned()))
                        .when(!task.blocked_by.is_empty(), |item| {
                            item.child(
                                gpui::div()
                                    .text_xs()
                                    .child(format!("blocked by: {}", task.blocked_by.join(", "))),
                            )
                        })
                        .when(!task.blocks.is_empty(), |item| {
                            item.child(
                                gpui::div()
                                    .text_xs()
                                    .child(format!("blocks: {}", task.blocks.join(", "))),
                            )
                        }),
                );
            }
            content = content.child(
                gpui::div()
                    .mt_3()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Mailbox ({})", self.state.mailbox.len())),
            );
            if self.state.mailbox.is_empty() {
                content = content.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("No mailbox messages"),
                );
            }
            for message in self.state.mailbox.iter().take(50) {
                let summary = message.summary.as_deref().unwrap_or(&message.content);
                let read_status = if message.read { "read" } else { "unread" };
                content = content.child(
                    gpui::div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "{} · {} → {} · {}",
                            message.msg_type, message.from_agent_id, message.to_agent_id, read_status
                        ))
                        .child(gpui::div().text_xs().child(summary.to_owned()))
                        .when(!message.files.is_empty(), |item| {
                            item.child(
                                gpui::div()
                                    .text_xs()
                                    .child(format!("files: {}", message.files.join(", "))),
                            )
                        }),
                );
            }
            content = content.child(
                gpui::div()
                    .mt_3()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Activity"),
            );
            for item in self.state.activity.iter().take(50) {
                let label = if item.kind == "message" {
                    item.message
                        .as_ref()
                        .map(|message| format!("message · {}", message.summary.as_deref().unwrap_or(&message.content)))
                } else {
                    item.task
                        .as_ref()
                        .map(|task| format!("task · {} · {}", task.subject, task.status))
                };
                if let Some(label) = label {
                    content = content.child(gpui::div().text_xs().text_color(theme.muted_foreground).child(label));
                }
            }
        }
        content
    }
}
