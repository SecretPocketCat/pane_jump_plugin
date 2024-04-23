use crate::{PluginState, ProjectTab};

impl PluginState {
    pub(crate) fn active_project(&self) -> Option<&ProjectTab> {
        self.tab.as_ref().and_then(|t| self.projects.get(t))
    }

    pub(crate) fn active_project_mut(&mut self) -> Option<&mut ProjectTab> {
        self.tab.as_ref().and_then(|t| self.projects.get_mut(t))
    }
}
