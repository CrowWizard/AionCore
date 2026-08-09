use gpui::{AppContext as _, Context, Entity, ParentElement as _, Styled, Window, px};
use gpui_component::{
    ActiveTheme, IconName, Sizable, WindowExt as _,
    button::Button,
    h_flex,
    input::{Input, InputState},
    label::Label,
    setting::{SettingField, SettingGroup, SettingItem, SettingPage},
    v_flex,
};
use rust_i18n::t;
use std::collections::HashMap;

use super::panel::SettingsPanel;
use crate::{
    AppState,
    app::actions::{
        AddAgent, ChangeConfigPath, ReloadAgentConfig, RemoveAgent, RestartAgent, UpdateAgent,
    },
};

impl SettingsPanel {
    pub fn agent_page(&self, view: &Entity<Self>) -> SettingPage {
        SettingPage::new(t!("settings.agents.title").to_string())
            .resettable(false)
            .groups(vec![
                SettingGroup::new()
                    .title(t!("settings.agents.group.configuration").to_string())
                    .items(vec![
                        SettingItem::new(
                            t!("settings.agents.config.path.label").to_string(),
                            SettingField::render({
                                let view = view.clone();
                                move |_options, _window, cx| {
                                    let config_path = AppState::global(cx)
                                        .agent_config_service()
                                        .map(|s| s.config_path().to_string_lossy().to_string())
                                        .unwrap_or_else(|| {
                                            t!("settings.agents.config.path.not_configured")
                                                .to_string()
                                        });

                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            gpui::div()
                                                .w_full()
                                                .overflow_x_hidden()
                                                .child(
                                                    Label::new(config_path)
                                                        .text_sm()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .whitespace_nowrap()
                                                )
                                        )
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .child(
                                                    Button::new("browse-config")
                                                        .label(
                                                            t!("settings.agents.config.path.browse")
                                                                .to_string(),
                                                        )
                                                        .icon(IconName::Folder)
                                                        .outline()
                                                        .small()
                                                        .on_click({
                                                            let view = view.clone();
                                                            move |_, window, cx| {
                                                                view.update(cx, |this, cx| {
                                                                    this.show_config_file_picker(window, cx);
                                                                });
                                                            }
                                                        })
                                                )
                                                .child(
                                                    Button::new("reload-config")
                                                        .label(
                                                            t!("settings.agents.config.path.reload")
                                                                .to_string(),
                                                        )
                                                        .icon(IconName::LoaderCircle)
                                                        .outline()
                                                        .small()
                                                        .on_click(move |_, window, cx| {
                                                            window.dispatch_action(
                                                                Box::new(ReloadAgentConfig),
                                                                cx
                                                            );
                                                        })
                                                )
                                        )
                                }
                            }),
                        )
                        .description(
                            t!("settings.agents.config.path.description").to_string(),
                        ),
                        SettingItem::new(
                            t!("settings.agents.upload_dir.label").to_string(),
                            SettingField::render({
                                let view = view.clone();
                                move |_options, _window, cx| {
                                    let upload_dir = view.read(cx).cached_upload_dir.to_string_lossy().to_string();
                                    let display = if upload_dir.is_empty() {
                                        t!("settings.agents.upload_dir.not_configured").to_string()
                                    } else {
                                        upload_dir
                                    };

                                    gpui::div()
                                        .w_full()
                                        .min_w(px(0.))
                                        .overflow_x_hidden()
                                        .child(
                                            Label::new(display)
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .whitespace_nowrap()
                                        )
                                }
                            }),
                        )
                        .description(
                            t!("settings.agents.upload_dir.description").to_string(),
                        ),
                    ]),
                SettingGroup::new()
                    .title(t!("settings.agents.group.configured").to_string())
                    .item(SettingItem::render({
                        let view = view.clone();
                        move |_options, _window, cx| {
                            let agent_configs = view.read(cx).cached_agents.clone();

                            let mut content = v_flex()
                                .w_full()
                                .gap_3()
                                .child(
                                    // Add New Agent button
                                    h_flex()
                                        .w_full()
                                        .justify_end()
                                        .child(
                                            Button::new("add-agent-btn")
                                                .label(
                                                    t!("settings.agents.button.add").to_string(),
                                                )
                                                .icon(IconName::Plus)
                                                .small()
                                                .on_click({
                                                    let view = view.clone();
                                                    move |_, window, cx| {
                                                        view.update(cx, |this, cx| {
                                                            this.show_add_edit_agent_dialog(window, cx, None);
                                                        });
                                                    }
                                                })
                                        )
                                );

                            if agent_configs.is_empty() {
                                content = content.child(
                                    h_flex()
                                        .w_full()
                                        .p_4()
                                        .justify_center()
                                        .child(
                                            Label::new(t!("settings.agents.empty").to_string())
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                        )
                                );
                            } else {
                                for (idx, (name, config)) in agent_configs.iter().enumerate() {
                                    let name_for_edit = name.clone();
                                    let name_for_restart = name.clone();
                                    let name_for_remove = name.clone();

                                    let mut agent_info = v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(
                                            Label::new(name.clone())
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                        )
                                        .child(
                                            Label::new(
                                                t!(
                                                    "settings.agents.field.command",
                                                    command = config.command
                                                )
                                                .to_string(),
                                            )
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                        );

                                    if !config.args.is_empty() {
                                        agent_info = agent_info.child(
                                            Label::new(
                                                t!(
                                                    "settings.agents.field.args",
                                                    args = config.args.join(" ")
                                                )
                                                .to_string(),
                                            )
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                        );
                                    }

                                    if !config.env.is_empty() {
                                        agent_info = agent_info.child(
                                            Label::new(
                                                t!(
                                                    "settings.agents.field.env",
                                                    count = config.env.len()
                                                )
                                                .to_string(),
                                            )
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                        );
                                    }

                                    content = content.child(
                                        h_flex()
                                            .w_full()
                                            .items_start()
                                            .justify_between()
                                            .p_3()
                                            .gap_3()
                                            .rounded(px(6.))
                                            .bg(cx.theme().secondary)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .child(agent_info)
                                            .child(
                                                // Action buttons column
                                                h_flex()
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Button::new(("edit-btn", idx))
                                                            .label(
                                                                t!("settings.agents.button.edit")
                                                                    .to_string(),
                                                            )
                                                            .icon(IconName::Settings)
                                                            .outline()
                                                            .small()
                                                            .on_click({
                                                                let view = view.clone();
                                                                move |_, window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.show_add_edit_agent_dialog(
                                                                            window,
                                                                            cx,
                                                                            Some(name_for_edit.clone())
                                                                        );
                                                                    });
                                                                }
                                                            })
                                                    )
                                                    .child(
                                                        Button::new(("restart-btn", idx))
                                                            .label(
                                                                t!("settings.agents.button.restart")
                                                                    .to_string(),
                                                            )
                                                            .icon(IconName::LoaderCircle)
                                                            .outline()
                                                            .small()
                                                            .on_click(move |_, window, cx| {
                                                                log::info!("Restart agent: {}", name_for_restart);
                                                                window.dispatch_action(
                                                                    Box::new(RestartAgent {
                                                                        name: name_for_restart.clone(),
                                                                    }),
                                                                    cx
                                                                );
                                                            })
                                                    )
                                                    .child(
                                                        Button::new(("remove-btn", idx))
                                                            .label(
                                                                t!("settings.agents.button.remove")
                                                                    .to_string(),
                                                            )
                                                            .icon(IconName::Delete)
                                                            .outline()
                                                            .small()
                                                            .on_click({
                                                                let view = view.clone();
                                                                move |_, window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.show_delete_confirm_dialog(
                                                                            window,
                                                                            cx,
                                                                            name_for_remove.clone()
                                                                        );
                                                                    });
                                                                }
                                                            })
                                                    )
                                            )
                                    );
                                }
                            }

                            content
                        }
                    })),
            ])
    }

    /// Show dialog to add or edit an agent
    pub fn show_add_edit_agent_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        agent_name: Option<String>,
    ) {
        let is_edit = agent_name.is_some();
        let title = if is_edit {
            t!("settings.agents.dialog.edit.title").to_string()
        } else {
            t!("settings.agents.dialog.add.title").to_string()
        };

        // Get existing config if editing
        let existing_config = agent_name
            .as_ref()
            .and_then(|name| self.cached_agents.get(name).cloned());

        // Create input states
        let name_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("settings.agents.input.name.placeholder").to_string());
            if let Some(name) = &agent_name {
                state.set_value(name.clone(), window, cx);
            }
            state
        });

        let command_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("settings.agents.input.command.placeholder").to_string());
            if let Some(config) = &existing_config {
                state.set_value(config.command.clone(), window, cx);
            }
            state
        });

        let args_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("settings.agents.input.args.placeholder").to_string());
            if let Some(config) = &existing_config {
                state.set_value(config.args.join(" "), window, cx);
            }
            state
        });

        let env_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder(t!("settings.agents.input.env.placeholder").to_string());
            if let Some(config) = &existing_config {
                let env_text = config
                    .env
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");
                state.set_value(env_text, window, cx);
            }
            state
        });

        window.open_dialog(cx, move |dialog, _window, cx| {
            dialog
                .title(title.clone())
                .footer(Self::confirm_footer(
                    if is_edit {
                        t!("settings.agents.dialog.edit.ok").to_string()
                    } else {
                        t!("settings.agents.dialog.add.ok").to_string()
                    },
                    t!("settings.agents.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Primary,
                ))
                .on_ok({
                    let name_input = name_input.clone();
                    let command_input = command_input.clone();
                    let args_input = args_input.clone();
                    let env_input = env_input.clone();
                    let _agent_name = agent_name.clone();

                    move |_, window, cx| {
                        let name = name_input.read(cx).text().to_string();
                        let name = name.trim();
                        let command = command_input.read(cx).text().to_string();
                        let command = command.trim();
                        let args_text = args_input.read(cx).text().to_string();
                        let env_text = env_input.read(cx).text().to_string();

                        // Validate inputs
                        if name.is_empty() || command.is_empty() {
                            log::warn!("Agent name and command cannot be empty");
                            return false;
                        }

                        // Parse args and env
                        let args: Vec<String> =
                            args_text.split_whitespace().map(String::from).collect();

                        let mut env = HashMap::new();
                        for line in env_text.lines() {
                            if let Some((key, value)) = line.trim().split_once('=') {
                                env.insert(key.trim().to_string(), value.trim().to_string());
                            } else if !line.trim().is_empty() {
                                log::warn!("Invalid env format (should be KEY=VALUE): {}", line);
                                return false;
                            }
                        }

                        // Dispatch appropriate action
                        if is_edit {
                            window.dispatch_action(
                                Box::new(UpdateAgent {
                                    name: name.to_string(),
                                    command: command.to_string(),
                                    args,
                                    env,
                                }),
                                cx,
                            );
                        } else {
                            window.dispatch_action(
                                Box::new(AddAgent {
                                    name: name.to_string(),
                                    command: command.to_string(),
                                    args,
                                    env,
                                }),
                                cx,
                            );
                        }

                        true // Close dialog
                    }
                })
                .child(
                    v_flex()
                        .w_full()
                        .gap_4()
                        .p_4()
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    Label::new(t!("settings.agents.field.name").to_string())
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD),
                                )
                                .child(
                                    Input::new(&name_input).disabled(is_edit), // Can't change name when editing
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    Label::new(
                                        t!("settings.agents.field.command_label").to_string(),
                                    )
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD),
                                )
                                .child(Input::new(&command_input))
                                .child(
                                    Label::new(
                                        t!("settings.agents.field.command_help").to_string(),
                                    )
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                                ),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    Label::new(t!("settings.agents.field.args_label").to_string())
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD),
                                )
                                .child(Input::new(&args_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    Label::new(t!("settings.agents.field.env_label").to_string())
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD),
                                )
                                .child(Input::new(&env_input))
                                .child(
                                    Label::new(t!("settings.agents.field.env_help").to_string())
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground),
                                ),
                        ),
                )
        });
    }

    /// Show confirmation dialog before deleting an agent
    pub fn show_delete_confirm_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        agent_name: String,
    ) {
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let name = agent_name.clone();
            dialog
                .title(t!("settings.agents.dialog.delete.title").to_string())
                .footer(Self::confirm_footer(
                    t!("settings.agents.dialog.delete.ok").to_string(),
                    t!("settings.agents.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Danger,
                ))
                .on_ok(move |_, window, cx| {
                    log::info!("Deleting agent: {}", name);
                    window.dispatch_action(Box::new(RemoveAgent { name: name.clone() }), cx);
                    true
                })
                .child(
                    v_flex().w_full().gap_2().p_4().child(
                        Label::new(
                            t!("settings.agents.dialog.delete.message", name = agent_name)
                                .to_string(),
                        )
                        .text_sm(),
                    ),
                )
        });
    }

    /// Show file picker to select config file
    pub fn show_config_file_picker(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let weak_entity = cx.entity().downgrade();

        // Use rfd to open file dialog
        cx.spawn(async move |_this, cx| {
            let task = rfd::AsyncFileDialog::new()
                .set_title(t!("settings.agents.config.dialog.title").to_string())
                .add_filter(
                    t!("settings.agents.config.dialog.filter_json").to_string(),
                    &["json"],
                )
                .set_file_name("config.json")
                .pick_file();

            if let Some(file) = task.await {
                let path = file.path().to_path_buf();
                log::info!("Selected config file: {:?}", path);

                // Dispatch action to change config path
                _ = cx.update(|cx| {
                    if let Some(entity) = weak_entity.upgrade() {
                        entity.update(cx, |_this, cx| {
                            cx.dispatch_action(&ChangeConfigPath { path });
                        });
                    }
                });
            }
        })
        .detach();
    }
}
