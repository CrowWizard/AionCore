use gpui::{AppContext as _, Context, Entity, ParentElement as _, Styled, Window, px};
use gpui_component::{
    ActiveTheme, IconName, Sizable, WindowExt as _,
    button::Button,
    h_flex,
    input::{Input, InputState},
    label::Label,
    setting::{SettingGroup, SettingItem, SettingPage},
    v_flex,
};
use rust_i18n::t;

use super::panel::SettingsPanel;
use crate::AppState;

impl SettingsPanel {
    pub fn command_page(&self, view: &Entity<Self>) -> SettingPage {
        SettingPage::new(t!("settings.commands.title").to_string())
            .resettable(false)
            .groups(vec![
            SettingGroup::new()
                .title(t!("settings.commands.group.custom").to_string())
                .item(SettingItem::render({
                    let view = view.clone();
                    move |_options, _window, cx| {
                        let command_configs = view.read(cx).cached_commands.clone();

                        let mut content = v_flex().w_full().gap_3().child(
                            h_flex().w_full().justify_end().child(
                                Button::new("add-command-btn")
                                    .label(t!("settings.commands.button.add").to_string())
                                    .icon(IconName::Plus)
                                    .small()
                                    .on_click({
                                        let view = view.clone();
                                        move |_, window, cx| {
                                            view.update(cx, |this, cx| {
                                                this.show_add_command_dialog(window, cx);
                                            });
                                        }
                                    }),
                            ),
                        );

                        if command_configs.is_empty() {
                            content = content.child(
                            h_flex().w_full().p_4().justify_center().child(
                                Label::new(
                                    t!("settings.commands.empty").to_string(),
                                )
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                            ),
                        );
                        } else {
                            for (idx, (name, config)) in command_configs.iter().enumerate() {
                                let name_for_edit = name.clone();
                                let name_for_delete = name.clone();

                                let command_info = v_flex()
                                    .flex_1()
                                    .gap_1()
                                    .child(
                                        Label::new(format!("/{}", name))
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                    )
                                    .child(
                                        Label::new(config.description.clone())
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    );

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
                                        .child(command_info)
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .child(
                                                    Button::new(("edit-command-btn", idx))
                                                        .label(t!("settings.commands.button.edit").to_string())
                                                        .icon(IconName::Settings)
                                                        .outline()
                                                        .small()
                                                        .on_click({
                                                            let view = view.clone();
                                                            move |_, window, cx| {
                                                                view.update(cx, |this, cx| {
                                                                    this.show_edit_command_dialog(
                                                                        window,
                                                                        cx,
                                                                        name_for_edit.clone(),
                                                                    );
                                                                });
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    Button::new(("delete-command-btn", idx))
                                                        .label(t!("settings.commands.button.delete").to_string())
                                                        .icon(IconName::Delete)
                                                        .outline()
                                                        .small()
                                                        .on_click({
                                                            let view = view.clone();
                                                            move |_, window, cx| {
                                                                view.update(cx, |this, cx| {
                                                                    this.show_delete_command_dialog(
                                                                    window,
                                                                    cx,
                                                                    name_for_delete.clone(),
                                                                );
                                                                });
                                                            }
                                                        }),
                                                ),
                                        ),
                                );
                            }
                        }

                        content
                    }
                })),
        ])
    }

    pub fn show_add_command_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.commands.input.name.placeholder").to_string())
        });
        let desc_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.commands.input.description.placeholder").to_string())
        });
        let template_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.commands.input.template.placeholder").to_string())
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title(t!("settings.commands.dialog.add.title").to_string())
                .footer(Self::confirm_footer(
                    t!("settings.commands.dialog.add.ok").to_string(),
                    t!("settings.commands.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Primary,
                ))
                .on_ok({
                    let name_input = name_input.clone();
                    let desc_input = desc_input.clone();
                    let template_input = template_input.clone();

                    move |_, _window, cx| {
                        let name = name_input.read(cx).text().to_string().trim().to_string();
                        let desc = desc_input.read(cx).text().to_string().trim().to_string();
                        let template = template_input
                            .read(cx)
                            .text()
                            .to_string()
                            .trim()
                            .to_string();

                        if name.is_empty() || desc.is_empty() || template.is_empty() {
                            log::warn!("Name, description, and template cannot be empty");
                            return false;
                        }

                        // Save to config file
                        if let Some(service) = AppState::global(cx).agent_config_service() {
                            let service = service.clone();
                            let config = crate::core::config::CommandConfig {
                                description: desc,
                                template,
                            };

                            cx.spawn(async move |cx| {
                                match service.add_command(name.clone(), config).await {
                                    Ok(_) => {
                                        log::info!("Successfully added command: {}", name);
                                        _ = cx.update(|_cx| {});
                                    }
                                    Err(e) => {
                                        log::error!("Failed to add command: {}", e);
                                    }
                                }
                            })
                            .detach();
                        }

                        true
                    }
                })
                .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .p_4()
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(t!("settings.commands.field.name").to_string()))
                                .child(Input::new(&name_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.commands.field.description").to_string(),
                                ))
                                .child(Input::new(&desc_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.commands.field.template").to_string(),
                                ))
                                .child(Input::new(&template_input)),
                        ),
                )
        });
    }

    pub fn show_edit_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        command_name: String,
    ) {
        let config = self.cached_commands.get(&command_name).cloned();
        if config.is_none() {
            log::warn!("Command config not found: {}", command_name);
            return;
        }
        let config = config.unwrap();
        let entity = cx.entity().downgrade();

        let desc_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.description.clone(), window, cx);
            state
        });
        let template_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.template.clone(), window, cx);
            state
        });

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title(t!("settings.commands.dialog.edit.title", name = command_name).to_string())
                .footer(Self::confirm_footer(
                    t!("settings.commands.dialog.edit.ok").to_string(),
                    t!("settings.commands.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Primary,
                ))
                .on_ok({
                    let desc_input = desc_input.clone();
                    let template_input = template_input.clone();
                    let command_name = command_name.clone();
                    let entity = entity.clone();

                    move |_, _window, cx| {
                        let desc = desc_input.read(cx).text().to_string().trim().to_string();
                        let template = template_input
                            .read(cx)
                            .text()
                            .to_string()
                            .trim()
                            .to_string();

                        if desc.is_empty() || template.is_empty() {
                            log::warn!("Description and template cannot be empty");
                            return false;
                        }

                        // Save to config file
                        if let Some(service) = AppState::global(cx).agent_config_service() {
                            let service = service.clone();
                            let command_name_for_async = command_name.clone();
                            let config = crate::core::config::CommandConfig {
                                description: desc,
                                template,
                            };
                            let entity = entity.clone();

                            cx.spawn(async move |cx| {
                                match service
                                    .update_command(&command_name_for_async, config.clone())
                                    .await
                                {
                                    Ok(_) => {
                                        log::info!(
                                            "Successfully updated command: {}",
                                            command_name_for_async
                                        );
                                        // Update UI
                                        _ = cx.update(|cx| {
                                            if let Some(panel) = entity.upgrade() {
                                                panel.update(cx, |this, cx| {
                                                    this.cached_commands
                                                        .insert(command_name_for_async, config);
                                                    cx.notify();
                                                });
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        log::error!("Failed to update command: {}", e);
                                    }
                                }
                            })
                            .detach();
                        }

                        true
                    }
                })
                .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .p_4()
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.commands.field.description").to_string(),
                                ))
                                .child(Input::new(&desc_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.commands.field.template").to_string(),
                                ))
                                .child(Input::new(&template_input)),
                        ),
                )
        });
    }

    pub fn show_delete_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        command_name: String,
    ) {
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let name = command_name.clone();
            dialog
                .title(t!("settings.commands.dialog.delete.title").to_string())
                .footer(Self::confirm_footer(
                    t!("settings.commands.dialog.delete.ok").to_string(),
                    t!("settings.commands.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Danger,
                ))
                .on_ok(move |_, _window, cx| {
                    if let Some(service) = AppState::global(cx).agent_config_service() {
                        let service = service.clone();
                        let name = name.clone();
                        cx.spawn(async move |cx| {
                            if let Err(e) = service.remove_command(&name).await {
                                log::error!("Failed to delete command: {}", e);
                            } else {
                                log::info!("Successfully deleted command: {}", name);
                            }
                            let _ = cx.update(|_cx| {});
                        })
                        .detach();
                    }
                    true
                })
                .child(
                    v_flex().w_full().gap_2().p_4().child(
                        Label::new(format!(
                            "{}",
                            t!(
                                "settings.commands.dialog.delete.message",
                                name = command_name
                            )
                        ))
                        .text_sm(),
                    ),
                )
        });
    }
}
