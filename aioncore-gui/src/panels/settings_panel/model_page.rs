use gpui::{
    AppContext as _, Context, Entity, ParentElement as _, Styled, Window, prelude::FluentBuilder,
    px,
};
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
    pub fn model_page(&self, view: &Entity<Self>) -> SettingPage {
        SettingPage::new(t!("settings.models.title").to_string())
            .resettable(false)
            .groups(vec![
                // Default AI Model Selection
                SettingGroup::new()
                    .title(t!("settings.models.default.title").to_string())
                    .description(t!("settings.models.default.description").to_string())
                    .item(SettingItem::render({
                        let view = view.clone();
                        move |_options, _window, cx| {
                            let model_configs = view.read(cx).cached_models.clone();
                            let ai_service = AppState::global(cx).ai_service();

                            let default_model = if let Some(service) = ai_service {
                                service.config.read().unwrap().default_model.clone()
                            } else {
                                None
                            };

                            if model_configs.is_empty() {
                                return v_flex().w_full().child(
                                    Label::new(t!("settings.models.default.empty").to_string())
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground),
                                );
                            }

                            let mut options_flex = h_flex().w_full().gap_2().flex_wrap();

                            let mut idx: usize = 0;
                            for (name, config) in model_configs.iter() {
                                if !config.enabled {
                                    continue;
                                }

                                let is_default = default_model.as_ref() == Some(name);
                                let name_clone = name.clone();

                                options_flex = options_flex.child(
                                    Button::new(("default-model-btn", idx))
                                        .label(name.clone())
                                        .when(is_default, |btn| btn.icon(IconName::Check))
                                        .when(!is_default, |btn| btn.outline())
                                        .small()
                                        .on_click({
                                            let view = view.clone();
                                            move |_, _window, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.set_default_model(name_clone.clone(), cx);
                                                });
                                            }
                                        }),
                                );

                                idx += 1;
                            }

                            v_flex().w_full().gap_2().child(options_flex)
                        }
                    })),
                // Model Providers List
                SettingGroup::new()
                    .title(t!("settings.models.providers.title").to_string())
                    .item(SettingItem::render({
                        let view = view.clone();
                        move |_options, _window, cx| {
                            let model_configs = view.read(cx).cached_models.clone();

                            let mut content = v_flex().w_full().gap_3().child(
                                h_flex().w_full().justify_end().child(
                                    Button::new("add-model-btn")
                                        .label(t!("settings.models.button.add").to_string())
                                        .icon(IconName::Plus)
                                        .small()
                                        .on_click({
                                            let view = view.clone();
                                            move |_, window, cx| {
                                                view.update(cx, |this, cx| {
                                                    this.show_add_model_dialog(window, cx);
                                                });
                                            }
                                        }),
                                ),
                            );

                            if model_configs.is_empty() {
                                content = content.child(
                                    h_flex().w_full().p_4().justify_center().child(
                                        Label::new(t!("settings.models.empty").to_string())
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground),
                                    ),
                                );
                            } else {
                                for (idx, (name, config)) in model_configs.iter().enumerate() {
                                    let name_for_edit = name.clone();
                                    let name_for_delete = name.clone();

                                    let mut model_info = v_flex()
                                        .flex_1()
                                        .gap_1()
                                        .child(
                                            Label::new(name.clone())
                                                .text_sm()
                                                .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            Label::new(
                                                t!(
                                                    "settings.models.field.provider",
                                                    provider = config.provider
                                                )
                                                .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Label::new(
                                                t!(
                                                    "settings.models.field.url",
                                                    url = config.base_url
                                                )
                                                .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        );

                                    if !config.model_name.is_empty() {
                                        model_info = model_info.child(
                                            Label::new(
                                                t!(
                                                    "settings.models.field.model_name",
                                                    model = config.model_name
                                                )
                                                .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
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
                                            .child(model_info)
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_center()
                                                    .child(
                                                        Label::new(if config.enabled {
                                                            t!("settings.models.status.enabled")
                                                                .to_string()
                                                        } else {
                                                            t!("settings.models.status.disabled")
                                                                .to_string()
                                                        })
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground),
                                                    )
                                                    .child(
                                                        Button::new(("edit-model-btn", idx))
                                                            .label(
                                                                t!("settings.models.button.edit")
                                                                    .to_string(),
                                                            )
                                                            .icon(IconName::Settings)
                                                            .outline()
                                                            .small()
                                                            .on_click({
                                                                let view = view.clone();
                                                                move |_, window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                        this.show_edit_model_dialog(
                                                                        window,
                                                                        cx,
                                                                        name_for_edit.clone(),
                                                                    );
                                                                    });
                                                                }
                                                            }),
                                                    )
                                                    .child(
                                                        Button::new(("delete-model-btn", idx))
                                                            .label(
                                                                t!("settings.models.button.delete")
                                                                    .to_string(),
                                                            )
                                                            .icon(IconName::Delete)
                                                            .outline()
                                                            .small()
                                                            .on_click({
                                                                let view = view.clone();
                                                                move |_, window, cx| {
                                                                    view.update(cx, |this, cx| {
                                                                    this.show_delete_model_dialog(
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

    pub fn show_add_model_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.models.input.name.placeholder").to_string())
        });
        let provider_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.models.input.provider.placeholder").to_string())
        });
        let url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.models.input.url.placeholder").to_string())
        });
        let key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.models.input.api_key.placeholder").to_string())
        });
        let model_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("settings.models.input.model_name.placeholder").to_string())
        });
        let entity = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title(t!("settings.models.dialog.add.title").to_string())
                .footer(Self::confirm_footer(
                    t!("settings.models.dialog.add.ok").to_string(),
                    t!("settings.models.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Primary,
                ))
                .on_ok({
                    let name_input = name_input.clone();
                    let provider_input = provider_input.clone();
                    let url_input = url_input.clone();
                    let key_input = key_input.clone();
                    let model_input = model_input.clone();
                    let entity = entity.clone();

                    move |_, _window, cx| {
                        let name = name_input.read(cx).text().to_string().trim().to_string();
                        let provider = provider_input
                            .read(cx)
                            .text()
                            .to_string()
                            .trim()
                            .to_string();
                        let url = url_input.read(cx).text().to_string().trim().to_string();
                        let key = key_input.read(cx).text().to_string().trim().to_string();
                        let model = model_input.read(cx).text().to_string().trim().to_string();

                        if name.is_empty() || provider.is_empty() || url.is_empty() {
                            log::warn!("Name, provider, and URL cannot be empty");
                            return false;
                        }

                        // Save to config file
                        if let Some(service) = AppState::global(cx).agent_config_service() {
                            let service = service.clone();
                            let config = crate::core::config::ModelConfig {
                                enabled: true,
                                provider,
                                base_url: url,
                                api_key: key,
                                model_name: model,
                            };
                            let name_clone = name.clone();
                            let entity = entity.clone();

                            cx.spawn(async move |cx| {
                                match service.add_model(name_clone.clone(), config.clone()).await {
                                    Ok(_) => {
                                        log::info!("Successfully added model: {}", name_clone);
                                        // Update UI
                                        _ = cx.update(|cx| {
                                            if let Some(panel) = entity.upgrade() {
                                                panel.update(cx, |this, cx| {
                                                    this.cached_models.insert(name_clone, config);
                                                    cx.notify();
                                                });
                                            }
                                        });
                                    }
                                    Err(e) => {
                                        log::error!("Failed to add model: {}", e);
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
                                .child(Label::new(t!("settings.models.field.name").to_string()))
                                .child(Input::new(&name_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.models.field.provider_label").to_string(),
                                ))
                                .child(Input::new(&provider_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.models.field.url_label").to_string(),
                                ))
                                .child(Input::new(&url_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(t!("settings.models.field.api_key").to_string()))
                                .child(Input::new(&key_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.models.field.model_name_label").to_string(),
                                ))
                                .child(Input::new(&model_input)),
                        ),
                )
        });
    }

    pub fn show_edit_model_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        model_name: String,
    ) {
        let Some(config) = self.cached_models.get(&model_name).cloned() else {
            log::warn!("Model config not found: {}", model_name);
            return;
        };

        let provider_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.provider.clone(), window, cx);
            state
        });
        let url_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.base_url.clone(), window, cx);
            state
        });
        let key_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.api_key.clone(), window, cx);
            state
        });
        let model_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_value(config.model_name.clone(), window, cx);
            state
        });

        let enabled = config.enabled;

        window.open_dialog(cx, move |dialog, _window, _cx| {
            dialog
                .title(t!("settings.models.dialog.edit.title", name = model_name).to_string())
                .footer(Self::confirm_footer(
                    t!("settings.models.dialog.edit.ok").to_string(),
                    t!("settings.models.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Primary,
                ))
                .on_ok({
                    let provider_input = provider_input.clone();
                    let url_input = url_input.clone();
                    let key_input = key_input.clone();
                    let model_input = model_input.clone();
                    let model_name = model_name.clone();

                    move |_, _window, cx| {
                        let provider = provider_input.read(cx).text().to_string();
                        let provider = provider.trim();
                        let url = url_input.read(cx).text().to_string();
                        let url = url.trim();
                        let key = key_input.read(cx).text().to_string();
                        let key = key.trim();
                        let model = model_input.read(cx).text().to_string();
                        let model = model.trim();

                        if provider.is_empty() || url.is_empty() {
                            log::warn!("Provider and URL cannot be empty");
                            return false;
                        }

                        if let Some(service) = AppState::global(cx).agent_config_service() {
                            let service = service.clone();
                            let name = model_name.clone();
                            let config = crate::core::config::ModelConfig {
                                enabled,
                                provider: provider.to_string(),
                                base_url: url.to_string(),
                                api_key: key.to_string(),
                                model_name: model.to_string(),
                            };

                            cx.spawn(async move |cx| {
                                if let Err(e) = service.update_model(&name, config).await {
                                    log::error!("Failed to update model: {}", e);
                                } else {
                                    log::info!("Successfully updated model: {}", name);
                                }
                                let _ = cx.update(|_cx| {});
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
                                    t!("settings.models.field.provider_label").to_string(),
                                ))
                                .child(Input::new(&provider_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.models.field.url_label").to_string(),
                                ))
                                .child(Input::new(&url_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(t!("settings.models.field.api_key").to_string()))
                                .child(Input::new(&key_input)),
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(Label::new(
                                    t!("settings.models.field.model_name_label").to_string(),
                                ))
                                .child(Input::new(&model_input)),
                        ),
                )
        });
    }

    pub fn show_delete_model_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        model_name: String,
    ) {
        window.open_dialog(cx, move |dialog, _window, _cx| {
            let name = model_name.clone();
            dialog
                .title(t!("settings.models.dialog.delete.title").to_string())
                .footer(Self::confirm_footer(
                    t!("settings.models.dialog.delete.ok").to_string(),
                    t!("settings.models.dialog.cancel").to_string(),
                    gpui_component::button::ButtonVariant::Danger,
                ))
                .on_ok(move |_, _window, cx| {
                    if let Some(service) = AppState::global(cx).agent_config_service() {
                        let service = service.clone();
                        let name = name.clone();
                        cx.spawn(async move |cx| {
                            if let Err(e) = service.remove_model(&name).await {
                                log::error!("Failed to delete model: {}", e);
                            } else {
                                log::info!("Successfully deleted model: {}", name);
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
                            t!("settings.models.dialog.delete.message", name = model_name)
                        ))
                        .text_sm(),
                    ),
                )
        });
    }

    pub fn set_default_model(&mut self, model_name: String, cx: &mut Context<Self>) {
        log::info!("Setting default AI model to: {}", model_name);

        // Update AI service default model
        if let Some(ai_service) = AppState::global(cx).ai_service() {
            let mut config = ai_service.config.write().unwrap();
            config.default_model = Some(model_name.clone());
            log::info!("Updated default model in AI service");
        }

        cx.notify();
    }
}
