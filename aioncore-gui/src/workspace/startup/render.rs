use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Disableable, IconName, Sizable, Size as UiSize, StyledExt as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use rust_i18n::t;

use crate::workspace::DockWorkspace;

impl DockWorkspace {
    pub(in crate::workspace) fn render_startup(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let workspace_ready = self.startup_state.workspace_ready();
        let workspace_icon = if workspace_ready {
            IconName::CircleCheck
        } else {
            IconName::Folder
        };

        let stepper = gpui_component::stepper::Stepper::new("startup-stepper")
            .w_full()
            .bg(cx.theme().background)
            .with_size(UiSize::Large)
            .selected_index(self.startup_state.step)
            .text_center(true)
            .items([
                gpui_component::stepper::StepperItem::new()
                    .icon(IconName::Settings)
                    .child(
                        v_flex()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t!("startup.step.preferences.title").to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .child(t!("startup.step.preferences.subtitle").to_string()),
                            ),
                    ),
                gpui_component::stepper::StepperItem::new().icon(workspace_icon).child(
                    v_flex()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(t!("startup.step.workspace.title").to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .child(t!("startup.step.workspace.subtitle").to_string()),
                        ),
                ),
            ])
            .on_click(cx.listener(|this, step, _, cx| {
                if !this.startup_state.intro_completed && *step > 0 {
                    return;
                }
                this.startup_state.step = *step;
                cx.notify();
            }));

        let content = if self.startup_state.step == 0 {
            self.render_preferences_step(cx)
        } else {
            self.render_workspace_step(cx)
        };

        div()
            .flex_1()
            .size_full()
            .bg(cx.theme().background)
            .flex()
            .items_center()
            .justify_center()
            .p_8()
            .child(v_flex().w_full().max_w(px(720.)).gap_8().child(stepper).child(content))
            .into_any_element()
    }

    fn render_preferences_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        v_flex()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(24.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(t!("startup.step.preferences.title").to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(theme.muted_foreground)
                            .child("AionCore GUI connects to the configured AionCore backend."),
                    ),
            )
            .child(
                h_flex().justify_end().child(
                    Button::new("startup-preferences-next")
                        .label("Continue")
                        .primary()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.startup_state.intro_completed = true;
                            this.startup_state.advance_step_if_needed();
                            cx.notify();
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_workspace_step(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let workspace_path = self
            .startup_state
            .workspace_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No workspace folder selected".to_owned());
        let button_label = if self.startup_state.workspace_selected {
            "Change folder"
        } else {
            "Choose folder"
        };

        v_flex()
            .gap_6()
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(24.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.foreground)
                            .child(t!("startup.step.workspace.title").to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(14.))
                            .text_color(theme.muted_foreground)
                            .child(workspace_path),
                    ),
            )
            .when_some(self.startup_state.workspace_error.clone(), |this, error| {
                this.child(div().text_sm().text_color(theme.danger).child(error))
            })
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        Button::new("startup-workspace-choose")
                            .label(button_label)
                            .outline()
                            .disabled(self.startup_state.workspace_loading)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_workspace_folder(window, cx);
                            })),
                    )
                    .child(
                        Button::new("startup-workspace-continue")
                            .label("Open AionCore GUI")
                            .primary()
                            .disabled(!self.startup_state.workspace_ready())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.startup_state.advance_step_if_needed();
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element()
    }
}
