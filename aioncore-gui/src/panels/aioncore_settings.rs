use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _, Render, RenderOnce,
    Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme,
    label::Label,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage, Settings},
    v_flex,
};

use crate::{AppSettings, AppState};

pub struct SettingsPanel {
    focus_handle: FocusHandle,
}

impl crate::panels::dock_panel::DockPanel for SettingsPanel {
    fn title() -> &'static str {
        "Settings"
    }

    fn title_key() -> Option<&'static str> {
        Some("settings.title")
    }

    fn description() -> &'static str {
        "AionCore GUI settings"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn paddings() -> gpui::Pixels {
        px(0.)
    }
}

impl SettingsPanel {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let panel = cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
        });
        Self::refresh(&panel, cx);
        panel
    }

    fn refresh(panel: &Entity<Self>, cx: &mut App) {
        let Some(store) = AppState::global(cx).settings_store().cloned() else {
            return;
        };
        let panel = panel.downgrade();
        cx.spawn(async move |cx| {
            let _ = store.refresh().await;
            let _ = cx.update(|cx| {
                if let Some(panel) = panel.upgrade() {
                    panel.update(cx, |_, cx| cx.notify());
                }
            });
        })
        .detach();
    }

    fn update_system_setting(&self, field: &'static str, value: bool, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).settings_store().cloned() else {
            return;
        };
        cx.spawn(async move |_, cx| {
            if let Err(error) = store.update_setting(field, serde_json::json!(value)).await {
                log::warn!("Failed to update AionCore setting {field}: {error:?}");
            }
            let _ = cx.update(|_| {});
        })
        .detach();
    }

    fn general_page(&self, view: &Entity<Self>) -> SettingPage {
        let panel = view.clone();
        SettingPage::new("通用").default_open(true).groups(vec![
            SettingGroup::new().title("界面").items(vec![
                SettingItem::new(
                    "深色模式",
                    SettingField::switch(
                        |cx: &App| cx.theme().mode.is_dark(),
                        |enabled, cx: &mut App| {
                            let mode = if enabled {
                                gpui_component::ThemeMode::Dark
                            } else {
                                gpui_component::ThemeMode::Light
                            };
                            gpui_component::Theme::change(mode, None, cx);
                        },
                    ),
                ),
                SettingItem::new(
                    "启动时检查更新",
                    SettingField::switch(
                        |cx: &App| AppSettings::global(cx).auto_check_on_startup,
                        |enabled, cx: &mut App| {
                            AppSettings::global_mut(cx).auto_check_on_startup = enabled;
                        },
                    ),
                ),
            ]),
            SettingGroup::new().title("AionCore").items(vec![
                SettingItem::new(
                    "消息通知",
                    SettingField::switch(
                        |cx: &App| {
                            AppState::global(cx)
                                .settings_store()
                                .and_then(|store| store.snapshot().settings)
                                .is_some_and(|settings| settings.notification_enabled)
                        },
                        {
                            let panel = panel.clone();
                            move |enabled, cx: &mut App| {
                                panel.update(cx, |panel, cx| {
                                    panel.update_system_setting("notification_enabled", enabled, cx);
                                });
                            }
                        },
                    ),
                ),
                SettingItem::new(
                    "上传文件保存到工作区",
                    SettingField::switch(
                        |cx: &App| {
                            AppState::global(cx)
                                .settings_store()
                                .and_then(|store| store.snapshot().settings)
                                .is_some_and(|settings| settings.save_upload_to_workspace)
                        },
                        {
                            let panel = panel.clone();
                            move |enabled, cx: &mut App| {
                                panel.update(cx, |panel, cx| {
                                    panel.update_system_setting("save_upload_to_workspace", enabled, cx);
                                });
                            }
                        },
                    ),
                ),
            ]),
        ])
    }

    fn proxy_page(&self) -> SettingPage {
        SettingPage::new("代理服务").groups(vec![SettingGroup::new().title("GUI 到 AionCore 的连接").items(vec![
                SettingItem::new(
                    "启用代理",
                    SettingField::switch(
                        |cx: &App| AppSettings::global(cx).proxy_enabled,
                        |enabled, cx: &mut App| {
                            AppSettings::global_mut(cx).proxy_enabled = enabled;
                            crate::app::themes::save_state(cx);
                        },
                    ),
                )
                .description("保存后在下次连接或重启 GUI 时生效。"),
                SettingItem::new(
                    "HTTP 代理",
                    SettingField::input(
                        |cx: &App| AppSettings::global(cx).http_proxy_url.clone().into(),
                        |value, cx: &mut App| {
                            AppSettings::global_mut(cx).http_proxy_url = value.to_string();
                            crate::app::themes::save_state(cx);
                        },
                    ),
                ),
                SettingItem::new(
                    "HTTPS 代理",
                    SettingField::input(
                        |cx: &App| AppSettings::global(cx).https_proxy_url.clone().into(),
                        |value, cx: &mut App| {
                            AppSettings::global_mut(cx).https_proxy_url = value.to_string();
                            crate::app::themes::save_state(cx);
                        },
                    ),
                ),
                SettingItem::new(
                    "通用代理",
                    SettingField::input(
                        |cx: &App| AppSettings::global(cx).all_proxy_url.clone().into(),
                        |value, cx: &mut App| {
                            AppSettings::global_mut(cx).all_proxy_url = value.to_string();
                            crate::app::themes::save_state(cx);
                        },
                    ),
                ),
            ])])
    }

    fn models_page(&self) -> SettingPage {
        SettingPage::new("模型").groups(vec![SettingGroup::new().title("Provider").item(SettingItem::render(
            |_, _, cx| {
                let providers = AppState::global(cx)
                    .settings_store()
                    .map(|store| store.snapshot().providers)
                    .unwrap_or_default();
                if providers.is_empty() {
                    return Label::new("未从 AionCore 获取到 Provider").text_sm().into_any_element();
                }
                v_flex()
                    .w_full()
                    .gap_2()
                    .children(providers.into_iter().map(|provider| {
                        let name = provider
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Unnamed");
                        let platform = provider
                            .get("platform")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown");
                        let enabled = provider
                            .get("enabled")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        Label::new(format!("{name} ({platform})"))
                            .text_sm()
                            .text_color(if enabled {
                                cx.theme().foreground
                            } else {
                                cx.theme().muted_foreground
                            })
                    }))
                    .into_any_element()
            },
        ))])
    }

    fn prompts_page(&self) -> SettingPage {
        SettingPage::new("提示词").groups(vec![SettingGroup::new().title("Skills").item(SettingItem::render(
            |_, _, cx| {
                let skills = AppState::global(cx)
                    .settings_store()
                    .map(|store| store.snapshot().skills)
                    .unwrap_or_default();
                if skills.is_empty() {
                    return Label::new("当前没有可用 Skills").text_sm().into_any_element();
                }
                v_flex()
                    .w_full()
                    .gap_2()
                    .children(
                        skills
                            .into_iter()
                            .map(|skill| Label::new(format!("{}: {}", skill.name, skill.description)).text_sm()),
                    )
                    .into_any_element()
            },
        ))])
    }

    fn mcp_page(&self) -> SettingPage {
        SettingPage::new("MCP 服务器").groups(vec![SettingGroup::new().title("已配置服务器").item(
            SettingItem::render(|_, _, cx| {
                let servers = AppState::global(cx)
                    .settings_store()
                    .map(|store| store.snapshot().mcp_servers)
                    .unwrap_or_default();
                if servers.is_empty() {
                    return Label::new("未从 AionCore 获取到 MCP 服务器")
                        .text_sm()
                        .into_any_element();
                }
                v_flex()
                    .w_full()
                    .gap_2()
                    .children(servers.into_iter().map(|server| {
                        let name = server
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Unnamed");
                        let enabled = server
                            .get("enabled")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        Label::new(name).text_sm().text_color(if enabled {
                            cx.theme().foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                    }))
                    .into_any_element()
            }),
        )])
    }

    fn commands_page(&self) -> SettingPage {
        SettingPage::new("命令").groups(vec![SettingGroup::new().title("Slash commands").item(
            SettingItem::render(|_, _, _| {
                Label::new("命令由当前 Conversation 的 agent 提供，在输入框键入 / 后显示。")
                    .text_sm()
                    .into_any_element()
            }),
        )])
    }
}

impl Focusable for SettingsPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Settings::new("aioncore-settings")
            .pages([
                self.general_page(&view),
                self.proxy_page(),
                self.models_page(),
                self.prompts_page(),
                self.mcp_page(),
                self.commands_page(),
            ])
            .render(window, cx)
    }
}
