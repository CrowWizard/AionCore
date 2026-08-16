use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _, Render, RenderOnce,
    Styled as _, Window, px,
};
use gpui_component::{
    ActiveTheme, Theme, ThemeRegistry,
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

    fn general_page(
        &self,
        view: &Entity<Self>,
        theme_options: Vec<(gpui::SharedString, gpui::SharedString)>,
    ) -> SettingPage {
        let panel = view.clone();
        SettingPage::new("通用").default_open(true).groups(vec![
            SettingGroup::new().title("界面").items(vec![
                SettingItem::new(
                    "主题",
                    SettingField::dropdown(
                        theme_options,
                        |cx: &App| cx.theme().theme_name().clone(),
                        |theme_name, cx: &mut App| {
                            if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
                                Theme::global_mut(cx).apply_config(&theme);
                                let font_size = AppSettings::global(cx).font_size;
                                Theme::global_mut(cx).font_size = gpui::px(font_size as f32);
                                crate::app::themes::save_state(cx);
                                cx.refresh_windows();
                            }
                        },
                    ),
                ),
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
                            crate::app::themes::save_state(cx);
                        },
                    ),
                ),
                SettingItem::new(
                    "自动切换主题",
                    SettingField::switch(
                        |cx: &App| AppSettings::global(cx).auto_switch_theme,
                        |enabled, cx: &mut App| {
                            AppSettings::global_mut(cx).auto_switch_theme = enabled;
                            crate::app::themes::save_state(cx);
                        },
                    ),
                ),
                SettingItem::new(
                    "启动时检查更新",
                    SettingField::switch(
                        |cx: &App| AppSettings::global(cx).auto_check_on_startup,
                        |enabled, cx: &mut App| {
                            AppSettings::global_mut(cx).auto_check_on_startup = enabled;
                            crate::app::themes::save_state(cx);
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
                SettingItem::new(
                    "Cron 通知",
                    SettingField::switch(
                        |cx: &App| {
                            AppState::global(cx)
                                .settings_store()
                                .and_then(|store| store.snapshot().settings)
                                .is_some_and(|settings| settings.cron_notification_enabled)
                        },
                        {
                            let panel = panel.clone();
                            move |enabled, cx: &mut App| {
                                panel.update(cx, |panel, cx| {
                                    panel.update_system_setting("cron_notification_enabled", enabled, cx);
                                });
                            }
                        },
                    ),
                ),
                SettingItem::new(
                    "命令队列",
                    SettingField::switch(
                        |cx: &App| {
                            AppState::global(cx)
                                .settings_store()
                                .and_then(|store| store.snapshot().settings)
                                .is_some_and(|settings| settings.command_queue_enabled)
                        },
                        {
                            let panel = panel.clone();
                            move |enabled, cx: &mut App| {
                                panel.update(cx, |panel, cx| {
                                    panel.update_system_setting("command_queue_enabled", enabled, cx);
                                });
                            }
                        },
                    ),
                ),
            ]),
            SettingGroup::new()
                .title("环境信息")
                .item(SettingItem::render(|_, _, cx| {
                    let current_directory = AppState::global(cx).current_working_dir().display().to_string();
                    let workspace_root = AppState::global(cx).current_working_dir().display().to_string();
                    let current_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S %:z").to_string();

                    v_flex()
                        .w_full()
                        .gap_2()
                        .children([
                            Label::new(format!("当前时间: {current_time}")).text_sm(),
                            Label::new(format!("工作目录: {current_directory}")).text_sm(),
                            Label::new(format!("Workspace 根目录: {workspace_root}")).text_sm(),
                        ])
                        .into_any_element()
                })),
        ])
    }

    fn proxy_page(&self) -> SettingPage {
        SettingPage::new("ACP 代理服务").groups(vec![SettingGroup::new().title("ACP agent 进程环境").item(
            SettingItem::render(|_, _, _| {
                Label::new("代理变量必须随 ACP agent 的命令、参数和环境变量保存到 AionCore agent runtime；不作用于 GUI 到 Core 的连接。")
                    .text_sm()
                    .into_any_element()
            }),
        )])
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
        SettingPage::new("ACP Agents").groups(vec![SettingGroup::new().title("已注册 agent runtime").item(
            SettingItem::render(|_, _, cx| {
                let agents = AppState::global(cx)
                    .settings_store()
                    .map(|store| store.snapshot().agents)
                    .unwrap_or_default();
                if agents.is_empty() {
                    return Label::new("未从 AionCore 获取到 agent runtime")
                        .text_sm()
                        .into_any_element();
                }
                v_flex()
                    .w_full()
                    .gap_2()
                    .children(agents.into_iter().map(|agent| {
                        let name = agent
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Unnamed");
                        let command = agent
                            .get("command")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("内置 agent");
                        let status = agent
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("unknown");
                        Label::new(format!("{name}: {command} ({status})")).text_sm()
                    }))
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
        let theme_options = ThemeRegistry::global(cx)
            .sorted_themes()
            .into_iter()
            .map(|theme| (theme.name.clone(), theme.name.clone()))
            .collect();
        Settings::new("aioncore-settings")
            .pages([
                self.general_page(&view, theme_options),
                self.proxy_page(),
                self.models_page(),
                self.prompts_page(),
                self.mcp_page(),
                self.commands_page(),
            ])
            .render(window, cx)
    }
}
