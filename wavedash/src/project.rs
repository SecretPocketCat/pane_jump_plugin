use crate::{PluginState, ProjectTab};

impl PluginState {
    pub(crate) fn active_project(&self) -> &ProjectTab {
        &self.projects[self.tab]
    }

    pub(crate) fn active_project_mut(&mut self) -> &mut ProjectTab {
        &mut self.projects[&self.tab]
    }
}
