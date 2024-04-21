use utils::pane::PaneId;
use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PaneManifest, TabInfo},
    shim::{get_plugin_ids, open_command_pane_floating, open_terminal_floating},
};

use crate::PluginState;

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
        // todo: needs a project-based rewrite
        if let Some(tab) = tabs.get(self.tab) {
            let floating = tab.are_floating_panes_visible;
            let proj = self.active_project_mut();
            if proj.floating != floating {
                proj.floating = floating;
                self.check_focus_change();
            }
        }

        // self.projects = tabs.iter().map(|t| (t.position, t.name.clone())).collect();
    }

    pub(crate) fn handle_pane_update(&mut self, PaneManifest { panes }: PaneManifest) {
        // if !self.check_itialised(&panes) {
        //     return;
        // }

        if let Some(tab_panes) = panes.get(&self.tab) {
            // collect all focused panes
            // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top
            self.active_project_mut().all_focused_panes =
                tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
            self.check_focus_change();

            for p in tab_panes {
                if p.terminal_command.is_some() && p.exit_status.is_some() {
                    let id = PaneId::from(p);

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

            let visible_panes: Vec<_> = tab_panes
                .iter()
                .filter(|p| {
                    p.is_selectable
                        && !p.is_floating
                        && PaneId::from(*p) != self.plugin_id
                        && !p.title.ends_with("-bar")
                        && p.title != "editor"
                })
                .collect();

            for pane in visible_panes {
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
