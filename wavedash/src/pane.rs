use utils::pane::{PaneFocus, PaneId};
use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PaneManifest, TabInfo},
    shim::{get_plugin_ids, hide_self, open_command_pane_floating, open_terminal_floating},
};

use crate::{PluginState, ProjectTab};

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
        if let Some(tab) = tabs.iter().find(|t| t.active) {
            self.tab = tab.position;
            let floating = tab.are_floating_panes_visible;
            if self.project_uninit() {
                hide_self();
                self.projects.insert(
                    tab.position,
                    ProjectTab {
                        title: tab.name.clone(),
                        editor_pane_id: None,
                        floating,
                        current_focus: None,
                        all_focused_panes: Default::default(),
                        status_panes: Default::default(),
                        terminal_panes: Default::default(),
                        keybind_panes: Default::default(),
                        spawned_extra_term_count: 0,
                    },
                );
            } else {
                let proj = self.active_project_mut();
                if proj.floating != floating {
                    proj.floating = floating;
                    self.check_focus_change();
                }
            }
        }
    }

    pub(crate) fn handle_pane_update(&mut self, PaneManifest { panes }: PaneManifest) {
        for (i, tab_panes) in panes.iter() {
            if self.project_uninit() {
                continue;
            }

            // collect all focused panes
            // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top
            self.active_project_mut().all_focused_panes =
                tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
            self.check_focus_change();

            if *i == self.tab {
                for p in tab_panes {
                    let id = PaneId::from(p);

                    if self.active_project().uninit() && p.title == "editor" {
                        self.active_project_mut().editor_pane_id = Some(id);
                    }

                    if p.terminal_command.is_some() && p.exit_status.is_some() {
                        if let Some((keybind_pane, id)) = self
                            .active_project()
                            .keybind_panes
                            .iter()
                            .find(|(_, v)| **v == id)
                            .map(|(k, v)| (*k, *v))
                        {
                            eprintln!("Removing keybind pane: {keybind_pane:?}, {id:?}");
                            self.active_project_mut()
                                .keybind_panes
                                .remove(&keybind_pane);
                            id.close();
                        }
                    }
                }

                let status_panes: Vec<_> = tab_panes
                    .iter()
                    .filter(|p| {
                        p.is_selectable
                            && !p.is_floating
                            && PaneId::from(*p) != self.plugin_id
                            && !p.title.ends_with("-bar")
                            && p.title != "editor"
                    })
                    .collect();

                for pane in status_panes {
                    self.active_project_mut()
                        .status_panes
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
}
