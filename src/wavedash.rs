use crate::{pane::PaneId, PluginState};

#[derive(Debug, Clone)]
pub(crate) struct DashPane {
    pub(crate) title: String,
    pub(crate) id: PaneId,
    pub(crate) editor: bool,
}

impl PluginState {}
