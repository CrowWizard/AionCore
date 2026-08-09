use gpui::*;
use std::path::PathBuf;

use crate::{AppState, utils};

use crate::workspace::DockWorkspace;

impl DockWorkspace {
    pub(in crate::workspace) fn ensure_startup_initialized(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.startup_state.initialized = true;
    }

    pub(in crate::workspace) fn open_workspace_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.startup_state.workspace_loading {
            return;
        }

        self.startup_state.workspace_loading = true;
        self.startup_state.workspace_error = None;
        cx.notify();

        cx.spawn_in(window, async move |this, window| {
            let selection = utils::pick_folder("Select workspace folder").await;
            let _ = this.update_in(window, |this, _, cx| {
                this.startup_state.workspace_loading = false;

                if let Some(path) = selection {
                    this.select_workspace_path(path, cx);
                }

                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::workspace) fn select_workspace_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !path.is_dir() {
            self.startup_state.workspace_error = Some("The selected path is not a directory.".to_owned());
            return;
        }

        self.startup_state.workspace_selected = true;
        self.startup_state.workspace_checked = true;
        self.startup_state.workspace_path = Some(path.clone());
        self.startup_state.workspace_error = None;
        AppState::global_mut(cx).set_current_working_dir(path);
        self.startup_state.advance_step_if_needed();
    }
}
