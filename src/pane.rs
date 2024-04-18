use phf::phf_map;
use std::{collections::HashSet, convert::TryFrom};
use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PaneInfo, PaneManifest, TabInfo},
    shim::{
        close_plugin_pane, close_terminal_pane, focus_plugin_pane, focus_terminal_pane,
        get_plugin_ids, hide_plugin_pane, hide_terminal_pane, open_command_pane_floating,
        open_terminal_floating, rename_plugin_pane, rename_terminal_pane,
    },
};

use crate::{
    message::{KeybindPane, DASH_CMD, YAZI_CMD},
    wavedash::DashPane,
    PluginState, PluginStatus,
};

pub(crate) const DASH_PANE_NAME: &str = "dash";
pub(crate) const FILEPICKER_PANE_NAME: &str = "filepicker";
pub(crate) const GIT_PANE_NAME: &str = "git";

static RENAME_PANE: phf::Map<&'static str, &'static str> = phf_map! {
    "lazygit" => GIT_PANE_NAME,
};
const RENAME_PANE_CONTAINS: [(&str, &str); 2] =
    [(DASH_CMD, DASH_PANE_NAME), (YAZI_CMD, FILEPICKER_PANE_NAME)];

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
            PaneFocus::Tiled(id) => id.clone(),
            PaneFocus::Floating(id) => id.clone(),
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

    pub(crate) fn map_dash_pane(&self, pane: &PaneInfo) -> DashPane {
        DashPane {
            id: pane.into(),
            title: pane.title.clone(),
        }
    }

    fn map_pane_name(&self, pane: &PaneInfo) -> Option<&str> {
        if let Some(name) = RENAME_PANE.get(&pane.title) {
            Some(name)
        } else if let Some((_, name)) = RENAME_PANE_CONTAINS
            .iter()
            .find(|(needle, _)| pane.title.contains(needle))
        {
            Some(name)
        } else {
            None
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
            for p in tab_panes {
                let id = PaneId::from(p);

                if let Some(new_name) = self.map_pane_name(p) {
                    id.rename(new_name);
                }

                if let Ok(keybind_pane) = KeybindPane::try_from(p) {
                    if p.terminal_command.is_some() && p.exit_status.is_some() {
                        id.close();
                        self.keybind_panes.remove(&keybind_pane);
                    } else {
                        self.keybind_panes.entry(keybind_pane).or_insert(id);
                    }
                }
            }

            match &self.status {
                crate::PluginStatus::FilePicker => {
                    if let Some(id) = self.keybind_panes.get(&KeybindPane::FilePicker) {
                        if let Some(file_picker_pane) =
                            tab_panes.iter().find(|p| &PaneId::from(*p) == id)
                        {
                            if file_picker_pane.exit_status.is_some() {
                                id.close();
                                self.status = PluginStatus::Editor;
                            }
                        }
                    }
                }
                _ => {
                    let visible_panes: Vec<_> = tab_panes
                        .iter()
                        .filter(|p| {
                            p.is_selectable
                                && PaneId::from(*p) != self.dash_pane_id
                                && !p.title.ends_with("-bar")
                        })
                        .collect();

                    let dash_pane_ids: HashSet<_> =
                        self.dash_panes.iter().map(|p| p.id.clone()).collect();

                    for pane in &visible_panes {
                        if dash_pane_ids.contains(&PaneId::from(*pane)) {
                            continue;
                        }

                        let dash_pane = self.map_dash_pane(pane);
                        // eprintln!("new dash pane: {dash_pane:?}");
                        self.dash_panes.push(dash_pane);
                    }

                    // cleanup closed panes
                    // todo: cleanup closed git pane etc
                    if self.dash_panes.len() > visible_panes.len() {
                        let visible_ids: HashSet<_> =
                            visible_panes.iter().map(|p| PaneId::from(*p)).collect();
                        self.dash_panes.retain(|p| visible_ids.contains(&p.id));
                    }

                    // collect all focused panes
                    // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top
                    self.all_focused_panes =
                        tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
                    self.check_focus_change();
                }
            }
        }
    }
}
