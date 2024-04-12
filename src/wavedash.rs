use crate::{pane::PaneId, PluginState};

#[derive(Debug, Clone)]
pub(crate) struct DashPane {
    pub(crate) title: String,
    pub(crate) id: PaneId,
    pub(crate) editor: bool,
}

impl PluginState {
    pub(crate) fn dash_pane_label_pairs(&self) -> Vec<(&DashPane, &str)> {
        self.dash_pane_labels
            .iter()
            .filter_map(|(label, id)| self.dash_panes.get(id).map(|p| (p, label.as_str())))
            .collect()
    }
}
