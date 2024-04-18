use crate::pane::PaneId;

#[derive(Debug, Clone)]
pub(crate) struct DashPane {
    pub(crate) title: String,
    pub(crate) id: PaneId,
}
