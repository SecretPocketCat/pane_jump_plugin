use std::collections::HashSet;

use phf::phf_map;
use zellij_tile::{
    prelude::{CommandToRun, PaneInfo, PaneManifest, TabInfo},
    shim::{
        close_plugin_pane, close_terminal_pane, focus_plugin_pane, focus_terminal_pane,
        hide_plugin_pane, hide_terminal_pane, open_command_pane, rename_plugin_pane,
        rename_terminal_pane,
    },
};

use crate::{file_picker::PickerStatus, wavedash::DashPane, PluginState, PluginStatus};

const GIT_PANE_NAME: &str = "git";

static RENAME_PANE: phf::Map<&'static str, &'static str> = phf_map! {
    "lazygit" => GIT_PANE_NAME,
};

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
    pub(crate) fn is_editor_pane(&self, pane: &PaneInfo) -> bool {
        !pane.is_floating
            && pane.is_selectable
            && (pane.pane_x == 0 && pane.pane_columns > (self.columns / 2)
                || pane.pane_y <= 2 && pane.pane_rows > (self.rows / 2))
    }

    pub(crate) fn map_pane(&self, pane: &PaneInfo) -> DashPane {
        DashPane {
            id: pane.into(),
            title: pane.title.clone(),
            editor: self.is_editor_pane(pane),
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
                if let Some(new_name) = RENAME_PANE.get(&p.title) {
                    let id = PaneId::from(p);
                    id.rename(new_name);

                    if *new_name == GIT_PANE_NAME {
                        self.git_pane_id = Some(id);
                    }
                }
            }

            match &self.status {
                crate::PluginStatus::FilePicker(picker_status) => match picker_status {
                    PickerStatus::OpeningPicker => {
                        if let Some(file_picker_pane) = panes.values().flatten().find(|p| {
                            p.terminal_command
                                .as_ref()
                                .is_some_and(|cmd| cmd.contains("yazi --chooser-file"))
                        }) {
                            rename_terminal_pane(file_picker_pane.id, "FilePicker");
                            self.status = PluginStatus::FilePicker(PickerStatus::Picking(
                                file_picker_pane.into(),
                            ));
                        }
                    }
                    PickerStatus::Picking(id) => {
                        if let Some(file_picker_pane) =
                            panes.values().flatten().find(|p| &PaneId::from(*p) == id)
                        {
                            if file_picker_pane.exit_status.is_some() {
                                id.close();
                                self.status = PluginStatus::Editor;
                            }
                        }
                    }
                    PickerStatus::Idle => {}
                },
                _ => {
                    // todo: maybe exclude floating?
                    let visible_panes: Vec<_> = tab_panes
                        .iter()
                        .filter(|p| {
                            p.is_selectable
                                && PaneId::from(*p) != self.dash_pane_id
                                && !p.title.ends_with("-bar")
                        })
                        .collect();

                    let dash_pane_ids: HashSet<_> =
                        self.dash_panes.values().map(|p| p.id.clone()).collect();

                    for pane in &visible_panes {
                        if dash_pane_ids.contains(&PaneId::from(*pane)) {
                            continue;
                        }

                        let dash_pane = self.map_pane(pane);
                        eprintln!("new dash pane: {dash_pane:?}");
                        self.dash_panes.insert(dash_pane.id.clone(), dash_pane);
                    }

                    // cleanup closed panes
                    // todo: cleanup closed git pane etc
                    let dash_panes_len = self.dash_panes.len();
                    if self.dash_panes.len() > visible_panes.len() {
                        let visible_ids: HashSet<_> =
                            visible_panes.iter().map(|p| PaneId::from(*p)).collect();
                        self.dash_panes.retain(|_, p| visible_ids.contains(&p.id));
                    }

                    let new_dash_panes_len = self.dash_panes.len();
                    if new_dash_panes_len < dash_panes_len && new_dash_panes_len > 0 {
                        if !self.dash_panes.contains_key(&self.current_focus.id()) {
                            // focus editor pane if the focused pane was closed
                            if let Some(editor_pane) =
                                self.dash_panes.values().filter(|p| p.editor).next()
                            {
                                editor_pane.id.focus();
                            }
                        }
                    } else if self.dash_panes.values().filter(|p| p.editor).count() == 0 {
                        // open a new editor pane if all editor panes were closed
                        eprintln!("No more editors");
                        open_command_pane(CommandToRun {
                            path: "hx".into(),
                            args: vec![".".to_string()],
                            cwd: None,
                        })
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
