use std::path::PathBuf;

use gpui::*;

#[derive(Debug)]
pub struct StartupState {
    pub(in crate::workspace) initialized: bool,
    pub(in crate::workspace) step: usize,
    pub(in crate::workspace) intro_completed: bool,
    pub(in crate::workspace) workspace_selected: bool,
    pub(in crate::workspace) workspace_path: Option<PathBuf>,
    pub(in crate::workspace) workspace_loading: bool,
    pub(in crate::workspace) workspace_error: Option<String>,
    pub(in crate::workspace) workspace_checked: bool,
    pub(in crate::workspace) workspace_check_in_progress: bool,
}

impl StartupState {
    pub(in crate::workspace) fn new() -> Self {
        Self {
            initialized: false,
            step: 0,
            intro_completed: false,
            workspace_selected: false,
            workspace_path: None,
            workspace_loading: false,
            workspace_error: None,
            workspace_checked: false,
            workspace_check_in_progress: false,
        }
    }

    pub(in crate::workspace) fn workspace_ready(&self) -> bool {
        true
    }

    pub(in crate::workspace) fn is_complete(&self) -> bool {
        self.intro_completed
    }

    pub(in crate::workspace) fn advance_step_if_needed(&mut self) {
        if self.step == 0 && self.intro_completed {
            self.step = 1;
        }
        if self.step > 1 {
            self.step = 1;
        }
    }
}
