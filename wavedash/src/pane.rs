use std::collections::HashSet;

use utils::{pane::PaneId, PROJECT_PICKER_PLUGIN_NAME};
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

    pub(crate) fn handle_queued_tab_update(&mut self) {
        if let Some(tab_update) = self.queued_tab_update.take() {
            self.handle_tab_update(&tab_update);
        }
    }

    fn handle_tab_update(&mut self, tabs: &[TabInfo]) {
        for (i, tab) in tabs.iter().enumerate() {
            // todo: looks like removing tabs causes the tabs to shift
            // closing projects/tabs should go through a custom function that will shift projects around

            if tab.name == PROJECT_PICKER_PLUGIN_NAME {
                continue;
            }

            let floating = tab.are_floating_panes_visible;
            if !self.projects.contains_key(&tab.name) {
                if self.projects.is_empty() {
                    // hide wavedash plugin (shown initially to confirm permissions)
                    hide_self();
                }

                eprintln!(
                    "New project '{}', position: {}, tab_keys: {:?}",
                    tab.name,
                    tab.position,
                    self.projects.keys()
                );

                self.projects.insert(
                    tab.name.clone(),
                    ProjectTab {
                        title: tab.name.clone(),
                        editor_pane_id: None,
                        floating,
                        current_focus: None,
                        status_panes: Default::default(),
                        terminal_panes: Default::default(),
                        keybind_panes: Default::default(),
                        spawned_extra_term_count: 0,
                    },
                );
            } else if tab.active {
                eprintln!(
                    "Changing active tab {:?}, pos: {}, i: {}, tab_keys: {:?}",
                    tab.name,
                    tab.position,
                    i,
                    self.projects.keys()
                );
                self.tab = Some(tab.name.clone());
                let proj = self.active_project_mut().unwrap();
                if proj.floating != floating {
                    proj.floating = floating;
                }
            }
        }

        let titles: HashSet<_> = tabs.iter().map(|t| &t.name).collect();
        self.projects.retain(|key, _| titles.contains(key));

        if let Some(pane_update) = self.queued_pane_update.take() {
            self.handle_pane_update(pane_update);
        }
    }

    fn handle_pane_update(&mut self, PaneManifest { panes }: PaneManifest) {
        if self.project_uninit() {
            return;
        }

        let tab_i = self
            .projects
            .get_index_of(self.tab.as_ref().unwrap())
            .unwrap();
        eprintln!("handling pane updates - tab_i: {tab_i}");

        // todo: just active tab
        for (i, tab_panes) in panes.iter() {
            // collect all focused panes
            // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top

            if *i == tab_i {
                eprintln!("Updating panes from focused tab '{:?}', {tab_i}", self.tab);
                let focused_panes: Vec<_> =
                    tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
                self.check_focus_change(&focused_panes);

                for p in tab_panes {
                    let id = PaneId::from(p);

                    if self.active_project().unwrap().uninit() && p.title == "editor" {
                        // todo: this is somehow incorrect and the editor_pane_id is shifted 1 tab/project over
                        // seems to happen when closing tabs - even the first tab (open project) causes this
                        eprintln!(
                            "Setting editor pane '{id:?}' to for tab {}",
                            self.active_project().unwrap().title
                        );
                        self.active_project_mut().unwrap().editor_pane_id = Some(id);
                    }

                    if p.terminal_command.is_some() && p.exit_status.is_some() {
                        if let Some((keybind_pane, id)) = self
                            .active_project()
                            .unwrap()
                            .keybind_panes
                            .iter()
                            .find(|(_, v)| **v == id)
                            .map(|(k, v)| (*k, *v))
                        {
                            eprintln!("Removing keybind pane: {keybind_pane:?}, {id:?}");
                            self.active_project_mut()
                                .unwrap()
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
                        .unwrap()
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
