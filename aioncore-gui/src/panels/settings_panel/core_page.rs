use gpui::{AppContext as _, Context, Entity, ParentElement as _, Styled, Window};
use gpui_component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    setting::{SettingGroup, SettingItem, SettingPage},
    v_flex,
};

use super::panel::SettingsPanel;
use crate::AppState;

impl SettingsPanel {
    pub fn core_page(&self, view: &Entity<Self>) -> SettingPage {
        SettingPage::new("AionCore")
            .resettable(false)
            .default_open(true)
            .groups(vec![
                SettingGroup::new().title("Server settings").item(SettingItem::render({
                    let view = view.clone();
                    move |_options, _window, cx| Self::render_core_settings(&view, cx)
                })),
                SettingGroup::new().title("Skills").item(SettingItem::render({
                    let view = view.clone();
                    move |_options, _window, cx| Self::render_skills(&view, cx)
                })),
                SettingGroup::new().title("Assistants").item(SettingItem::render({
                    let view = view.clone();
                    move |_options, _window, cx| Self::render_assistants(&view, cx)
                })),
                SettingGroup::new()
                    .title("Provider and MCP security")
                    .item(SettingItem::render(|_options, _window, cx| {
                        Label::new(
                            "Provider and MCP lists are unavailable until AionCore exposes redacted response DTOs.",
                        )
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                    })),
            ])
    }

    fn render_core_settings(view: &Entity<Self>, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let Some(store) = AppState::global(cx).settings_store().cloned() else {
            return Label::new("AionCore is disconnected").into_any_element();
        };
        let state = store.snapshot();
        let content = state.settings.map(|settings| {
            let enabled = settings.notification_enabled;
            v_flex()
                .w_full()
                .gap_3()
                .child(Label::new(format!("Language: {}", settings.language)).text_sm())
                .child(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .child(
                            Label::new(format!(
                                "Notifications: {}",
                                if enabled { "enabled" } else { "disabled" }
                            ))
                            .text_sm(),
                        )
                        .child(
                            Button::new("toggle-core-notifications")
                                .label(if enabled { "Disable" } else { "Enable" })
                                .outline()
                                .small()
                                .on_click({
                                    let view = view.clone();
                                    move |_, _window, cx| {
                                        let Some(store) = AppState::global(cx).settings_store().cloned() else {
                                            return;
                                        };
                                        let view = view.clone();
                                        cx.spawn(async move |cx| {
                                            let _ = store.set_notification_enabled(!enabled).await;
                                            let _ = cx.update(|cx| {
                                                if let Some(view) = view.upgrade() {
                                                    view.update(cx, |_, cx| cx.notify());
                                                }
                                            });
                                        })
                                        .detach();
                                    }
                                }),
                        ),
                )
                .into_any_element()
        });
        content.unwrap_or_else(|| {
            Label::new(if state.is_loading {
                "Loading AionCore settings…"
            } else {
                "Settings are unavailable"
            })
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .into_any_element()
        })
    }

    fn render_skills(_view: &Entity<Self>, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let skills = AppState::global(cx)
            .settings_store()
            .map(|store| store.snapshot().skills)
            .unwrap_or_default();
        if skills.is_empty() {
            return Label::new("No skills returned by AionCore")
                .text_sm()
                .into_any_element();
        }
        v_flex()
            .w_full()
            .gap_2()
            .children(
                skills
                    .into_iter()
                    .map(|skill| Label::new(format!("{} — {}", skill.name, skill.description)).text_sm()),
            )
            .into_any_element()
    }

    fn render_assistants(_view: &Entity<Self>, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let assistants = AppState::global(cx)
            .settings_store()
            .map(|store| store.snapshot().assistants)
            .unwrap_or_default();
        if assistants.is_empty() {
            return Label::new("No assistants returned by AionCore")
                .text_sm()
                .into_any_element();
        }
        v_flex()
            .w_full()
            .gap_2()
            .children(assistants.into_iter().map(|assistant| {
                Label::new(format!(
                    "{} ({})",
                    assistant.name,
                    if assistant.enabled { "enabled" } else { "disabled" }
                ))
                .text_sm()
            }))
            .into_any_element()
    }

    pub fn refresh_core_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).settings_store().cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        cx.spawn_in(window, async move |_this, window| {
            let _ = store.refresh().await;
            let _ = window.update(|_window, cx| {
                if let Some(view) = view.upgrade() {
                    view.update(cx, |_, cx| cx.notify());
                }
            });
        })
        .detach();
    }
}
