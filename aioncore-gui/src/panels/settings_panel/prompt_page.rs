use gpui::{Context, Entity, ParentElement as _, Styled, Window, prelude::FluentBuilder};
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::Button,
    h_flex,
    input::Input,
    label::Label,
    setting::{SettingGroup, SettingItem, SettingPage},
    v_flex,
};
use rust_i18n::t;

use super::panel::SettingsPanel;
use crate::AppState;

impl SettingsPanel {
    pub fn prompt_page(&self, view: &Entity<Self>) -> SettingPage {
        SettingPage::new(t!("settings.prompts.title").to_string())
            .resettable(false)
            .groups(vec![
                // Default AI Model Selection
                SettingGroup::new()
                    .title(t!("settings.prompts.default.title").to_string())
                    .description(t!("settings.prompts.default.description").to_string())
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
                                    Label::new(t!("settings.prompts.default.empty").to_string())
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
                                    Button::new(("default-model-btn-prompt", idx))
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
                // System Prompts Configuration
                SettingGroup::new()
                    .title(t!("settings.prompts.system.title").to_string())
                    .description(t!("settings.prompts.system.description").to_string())
                    .item(SettingItem::render({
                        let view = view.clone();
                        move |_options, _window, cx| {
                            // Read current prompt values from panel's InputStates
                            let doc_comment_state = view.read(cx).doc_comment_input.clone();
                            let inline_comment_state = view.read(cx).inline_comment_input.clone();
                            let explain_state = view.read(cx).explain_input.clone();
                            let improve_state = view.read(cx).improve_input.clone();

                            v_flex()
                                .w_full()
                                .gap_4()
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.doc.label").to_string(),
                                            )
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.doc.help").to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(Input::new(&doc_comment_state)),
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.inline.label")
                                                    .to_string(),
                                            )
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.inline.help")
                                                    .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(Input::new(&inline_comment_state)),
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.explain.label")
                                                    .to_string(),
                                            )
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.explain.help")
                                                    .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(Input::new(&explain_state)),
                                )
                                .child(
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.improve.label")
                                                    .to_string(),
                                            )
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD),
                                        )
                                        .child(
                                            Label::new(
                                                t!("settings.prompts.system.improve.help")
                                                    .to_string(),
                                            )
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(Input::new(&improve_state)),
                                )
                                .child(
                                    h_flex().w_full().justify_end().child(
                                        Button::new("save-prompts-btn")
                                            .label(t!("settings.prompts.button.save").to_string())
                                            .icon(IconName::Check)
                                            .small()
                                            .on_click({
                                                let view = view.clone();
                                                move |_, _window, cx| {
                                                    view.update(cx, |this, cx| {
                                                        this.save_system_prompts(cx);
                                                    });
                                                }
                                            }),
                                    ),
                                )
                        }
                    })),
            ])
    }

    pub fn save_system_prompts(&mut self, cx: &mut Context<Self>) {
        let doc_comment = self
            .doc_comment_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();
        let inline_comment = self
            .inline_comment_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();
        let explain = self
            .explain_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();
        let improve = self
            .improve_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();

        let mut system_prompts = std::collections::HashMap::new();
        if !doc_comment.is_empty() {
            system_prompts.insert("doc_comment".to_string(), doc_comment);
        }
        if !inline_comment.is_empty() {
            system_prompts.insert("inline_comment".to_string(), inline_comment);
        }
        if !explain.is_empty() {
            system_prompts.insert("explain".to_string(), explain);
        }
        if !improve.is_empty() {
            system_prompts.insert("improve".to_string(), improve);
        }

        // Save to config file via AgentConfigService
        if let Some(service) = AppState::global(cx).agent_config_service() {
            let service = service.clone();
            cx.spawn(async move |_this, cx| {
                match service.update_system_prompts(system_prompts.clone()).await {
                    Ok(_) => {
                        log::info!("Successfully saved system prompts");
                        // Update AI service config
                        _ = cx.update(|cx| {
                            if let Some(ai_service) = AppState::global(cx).ai_service() {
                                let models = ai_service.config.read().unwrap().models.clone();
                                ai_service.update_config(models, system_prompts);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to save system prompts: {}", e);
                    }
                }
            })
            .detach();
        }
    }

    /// Load system prompts from config into InputStates
    pub fn load_system_prompts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(ai_service) = AppState::global(cx).ai_service() {
            let system_prompts = ai_service.config.read().unwrap().system_prompts.clone();

            let doc_comment_input = self.doc_comment_input.clone();
            let inline_comment_input = self.inline_comment_input.clone();
            let explain_input = self.explain_input.clone();
            let improve_input = self.improve_input.clone();

            if let Some(prompt) = system_prompts.get("doc_comment") {
                let prompt = prompt.clone();
                doc_comment_input.update(cx, |state, cx| {
                    state.set_value(prompt, window, cx);
                });
            }
            if let Some(prompt) = system_prompts.get("inline_comment") {
                let prompt = prompt.clone();
                inline_comment_input.update(cx, |state, cx| {
                    state.set_value(prompt, window, cx);
                });
            }
            if let Some(prompt) = system_prompts.get("explain") {
                let prompt = prompt.clone();
                explain_input.update(cx, |state, cx| {
                    state.set_value(prompt, window, cx);
                });
            }
            if let Some(prompt) = system_prompts.get("improve") {
                let prompt = prompt.clone();
                improve_input.update(cx, |state, cx| {
                    state.set_value(prompt, window, cx);
                });
            }
        }
    }
}
