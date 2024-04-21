use crate::{PluginState, ProjectTab};

impl PluginState {
    pub(crate) fn active_project(&self) -> &ProjectTab {
        self.projects.get(&self.tab).expect("Got active project")
    }

    pub(crate) fn active_project_mut(&mut self) -> &mut ProjectTab {
        self.projects
            .get_mut(&self.tab)
            .expect("Got active project")
    }
}
