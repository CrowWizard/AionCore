use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement as _, Render, Styled, Window,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::{AppState, core::aioncore::ProjectState, panels::dock_panel::DockPanel};

pub struct ProjectPanel {
    focus_handle: FocusHandle,
    state: ProjectState,
}

impl DockPanel for ProjectPanel {
    fn title() -> &'static str {
        "Project"
    }
    fn description() -> &'static str {
        "AionCore project explorer"
    }
    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
    fn paddings() -> gpui::Pixels {
        px(12.)
    }
}

impl ProjectPanel {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            state: ProjectState::default(),
        };
        panel.refresh(cx);
        panel
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).project_store().cloned() else {
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

    fn select(&mut self, project_id: String, cx: &mut Context<Self>) {
        let Some(store) = AppState::global(cx).project_store().cloned() else {
            return;
        };
        let view = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let _ = store.select(&project_id).await;
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
}

impl Focusable for ProjectPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProjectPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let mut content = v_flex().size_full().gap_2().bg(theme.background).child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    gpui::div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("Project"),
                )
                .child(
                    Button::new("refresh-project")
                        .icon(Icon::new(IconName::LoaderCircle))
                        .ghost()
                        .small()
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                ),
        );
        if let Some(error) = &self.state.error {
            content = content.child(gpui::div().text_sm().text_color(theme.danger).child(error.clone()));
        }
        for (index, project) in self.state.projects.iter().enumerate() {
            let project_id = project.project_id.clone();
            let selected = self.state.selected_project_id.as_deref() == Some(project.project_id.as_str());
            content = content.child(
                Button::new(("project", index))
                    .label(format!("{} · {}", project.name, project.kind))
                    .when(selected, |button| button.primary())
                    .when(!selected, |button| button.outline())
                    .small()
                    .on_click(cx.listener(move |this, _, _, cx| this.select(project_id.clone(), cx))),
            );
        }
        if self.state.projects.is_empty() && !self.state.is_loading {
            content = content.child(
                gpui::div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("No projects returned by AionCore"),
            );
        }
        if let Some(project) = &self.state.project {
            content = content.child(
                gpui::div()
                    .mt_3()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(project.name.clone()),
            );
            for entry in &project.explorer.entries {
                content = content.child(gpui::div().text_xs().text_color(theme.muted_foreground).child(format!(
                    "{} · {}",
                    entry.display_name.as_deref().unwrap_or(&entry.display_path),
                    entry.runtime_status
                )));
            }
        }
        for file in self.state.files.iter().take(100) {
            content = content.child(gpui::div().text_xs().child(file.relative_path.clone()));
        }
        content
    }
}
