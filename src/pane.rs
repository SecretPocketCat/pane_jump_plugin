use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PaneInfo, PaneManifest, TabInfo},
    shim::{
        close_plugin_pane, close_terminal_pane, focus_plugin_pane, focus_terminal_pane,
        get_plugin_ids, hide_plugin_pane, hide_terminal_pane, open_command_pane_floating,
        open_terminal_floating, rename_plugin_pane, rename_terminal_pane,
    },
};

use crate::PluginState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PaneId {
    Terminal(u32),
    Plugin(u32),
}

impl PaneId {
    pub(crate) fn new(id: u32, plugin: bool) -> Self {
        if plugin {
            Self::Plugin(id)
        } else {
            Self::Terminal(id)
        }
    }

    pub(crate) fn focus(&self) {
        match self {
            PaneId::Terminal(id) => focus_terminal_pane(*id, false),
            PaneId::Plugin(id) => focus_plugin_pane(*id, false),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn hide(&self) {
        match self {
            PaneId::Terminal(id) => hide_terminal_pane(*id),
            PaneId::Plugin(id) => hide_plugin_pane(*id),
        }
    }

    pub(crate) fn close(&self) {
        match self {
            PaneId::Terminal(id) => close_terminal_pane(*id),
            PaneId::Plugin(id) => close_plugin_pane(*id),
        }
    }

    pub(crate) fn rename(&self, new_name: &str) {
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
pub(crate) enum PaneFocus {
    Tiled(PaneId),
    Floating(PaneId),
}

impl PaneFocus {
    pub(crate) fn new(id: impl Into<PaneId>, floating: bool) -> Self {
        if floating {
            Self::Floating(id.into())
        } else {
            Self::Tiled(id.into())
        }
    }

    pub(crate) fn id(&self) -> PaneId {
        match self {
            PaneFocus::Tiled(id) => *id,
            PaneFocus::Floating(id) => *id,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn floating(&self) -> bool {
        matches!(self, PaneFocus::Floating(_))
    }
}

impl From<&PaneInfo> for PaneFocus {
    fn from(pane: &PaneInfo) -> Self {
        Self::new(pane, pane.is_floating)
    }
}

impl PluginState {
    pub(crate) fn open_floating_pane(command: Option<CommandToRun>) {
        let coords = Some(
            FloatingPaneCoordinates::default()
                .with_x_fixed(0)
                .with_y_fixed(0)
                .with_width_percent(100)
                .with_height_percent(100),
        );

        if let Some(cmd) = command {
            open_command_pane_floating(cmd, coords);
        } else {
            open_terminal_floating(get_plugin_ids().initial_cwd, coords);
        }
    }

    pub(crate) fn handle_tab_update(&mut self, tabs: &[TabInfo]) {
        if let Some(tab) = tabs.get(self.tab) {
            let floating = tab.are_floating_panes_visible;
            if self.floating != floating {
                self.floating = floating;
                self.check_focus_change();
            }
        }
    }

    pub(crate) fn handle_pane_update(&mut self, PaneManifest { panes }: PaneManifest) {
        if !self.check_itialised(&panes) {
            return;
        }

        if let Some(tab_panes) = panes.get(&self.tab) {
            // collect all focused panes
            // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top
            self.all_focused_panes = tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
            self.check_focus_change();

            for p in tab_panes {
                if p.terminal_command.is_some() && p.exit_status.is_some() {
                    let id = PaneId::from(p);

                    if let Some((keybind_pane, id)) = self
                        .keybind_panes
                        .iter()
                        .find(|(_, v)| **v == id)
                        .map(|(k, v)| (*k, *v))
                    {
                        eprintln!("Removing keybind pane: {keybind_pane:?}, {id:?}");
                        self.keybind_panes.remove(&keybind_pane);
                        id.close();
                    }
                }
            }

            let visible_panes: Vec<_> = tab_panes
                .iter()
                .filter(|p| {
                    p.is_selectable
                        && !p.is_floating
                        && PaneId::from(*p) != self.dash_pane_id
                        && !p.title.ends_with("-bar")
                        && p.title != "editor"
                })
                .collect();

            for pane in visible_panes {
                self.status_panes
                    .entry(pane.into())
                    .and_modify(|t| {
                        if t != &pane.title {
                            *t = pane.title.clone();
                        }
                    })
                    .or_insert_with(|| pane.title.clone());
            }
        }
    }
}
