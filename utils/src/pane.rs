use zellij_tile::{
    prelude::PaneInfo,
    shim::{
        close_plugin_pane, close_terminal_pane, focus_plugin_pane, focus_terminal_pane,
        hide_plugin_pane, hide_terminal_pane, rename_plugin_pane, rename_terminal_pane,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaneId {
    Terminal(u32),
    Plugin(u32),
}

impl PaneId {
    pub fn new(id: u32, plugin: bool) -> Self {
        if plugin {
            Self::Plugin(id)
        } else {
            Self::Terminal(id)
        }
    }

    pub fn focus(&self) {
        match self {
            PaneId::Terminal(id) => focus_terminal_pane(*id, false),
            PaneId::Plugin(id) => focus_plugin_pane(*id, false),
        }
    }

    #[allow(dead_code)]
    pub fn hide(&self) {
        match self {
            PaneId::Terminal(id) => hide_terminal_pane(*id),
            PaneId::Plugin(id) => hide_plugin_pane(*id),
        }
    }

    pub fn close(&self) {
        match self {
            PaneId::Terminal(id) => close_terminal_pane(*id),
            PaneId::Plugin(id) => close_plugin_pane(*id),
        }
    }

    pub fn rename(&self, new_name: &str) {
        match self {
            PaneId::Terminal(id) => rename_terminal_pane(*id, new_name),
            PaneId::Plugin(id) => rename_plugin_pane(*id, new_name),
        }
    }
}

impl From<&PaneInfo> for PaneId {
    fn from(pane: &PaneInfo) -> Self {
        Self::new(pane.id, pane.is_plugin)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneFocus {
    Tiled(PaneId),
    Floating(PaneId),
}

impl PaneFocus {
    pub fn new(id: impl Into<PaneId>, floating: bool) -> Self {
        if floating {
            Self::Floating(id.into())
        } else {
            Self::Tiled(id.into())
        }
    }

    pub fn id(&self) -> PaneId {
        match self {
            PaneFocus::Tiled(id) => *id,
            PaneFocus::Floating(id) => *id,
        }
    }

    #[allow(dead_code)]
    pub fn floating(&self) -> bool {
        matches!(self, PaneFocus::Floating(_))
    }
}

impl From<&PaneInfo> for PaneFocus {
    fn from(pane: &PaneInfo) -> Self {
        Self::new(pane, pane.is_floating)
    }
}
