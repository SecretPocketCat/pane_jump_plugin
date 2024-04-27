use std::collections::HashSet;

use tracing::{debug, info, instrument, warn};
use utils::{pane::PaneId, project::PROJECT_ROOT_RQST_MESSAGE_NAME, PROJECT_PICKER_PLUGIN_NAME};
use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, MessageToPlugin, PaneManifest, TabInfo},
    shim::{
        get_plugin_ids, hide_self, open_command_pane_floating, open_terminal_floating,
        pipe_message_to_plugin,
    },
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

    #[instrument(skip_all)]
    fn handle_tab_update(&mut self, tabs: &[TabInfo]) {
        let panes: Vec<_> = tabs
            .iter()
            .map(|t| {
                (
                    &t.name,
                    t.active,
                    t.position,
                    self.projects
                        .get_index(t.position)
                        .and_then(|(_, t)| t.editor_pane_id),
                )
            })
            .collect();
        warn!(?panes, "panes");

        for (i, tab) in tabs.iter().enumerate() {
            if tab.name == PROJECT_PICKER_PLUGIN_NAME {
                continue;
            }

            let floating = tab.are_floating_panes_visible;
            if !self.projects.contains_key(&tab.name) {
                if self.projects.is_empty() {
                    // hide wavedash plugin (shown initially to confirm permissions)
                    hide_self();

                    // request project root
                    let msg = MessageToPlugin::new(PROJECT_ROOT_RQST_MESSAGE_NAME);
                    pipe_message_to_plugin(msg);
                }

                info!(
                    tab.name,tab.position,project_keys=?self.projects.keys(),
                    "New project",
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
                debug!(
                    tab.name, tab.position, i, proj_keys=?self.projects.keys(),
                    "Changing active tab",
                );

                self.tab = Some(tab.name.clone());
                let proj = self.active_project_mut().unwrap();
                proj.floating = floating;
            }
        }

        let titles: HashSet<_> = tabs.iter().map(|t| &t.name).collect();
        self.projects.retain(|key, _| titles.contains(key));

        if let Some(pane_update) = self.queued_pane_update.take() {
            self.handle_pane_update(pane_update);
        }
    }

    #[instrument(skip_all)]
    fn handle_pane_update(&mut self, PaneManifest { panes }: PaneManifest) {
        if self.project_uninit() {
            return;
        }

        let tab_i = self
            .projects
            .get_index_of(self.tab.as_ref().unwrap())
            .unwrap();
        debug!(tab_i, "handling pane updates");

        // todo: just active tab
        for (i, tab_panes) in panes.iter() {
            // collect all focused panes
            // this is used due to possible race conditions with `TabUpdate` which is used to update whether floating panes are on top

            if *i == tab_i {
                debug!(self.tab, tab_i, "Updating panes from focused tab");
                let focused_panes: Vec<_> =
                    tab_panes.iter().filter(|p| p.is_focused).cloned().collect();
                self.check_focus_change(&focused_panes);

                for p in tab_panes {
                    let id = PaneId::from(p);

                    if self.active_project().unwrap().uninit() && p.title == "editor" {
                        // todo: this is somehow incorrect and the editor_pane_id is shifted 1 tab/project over
                        // seems to happen when closing tabs - even the first tab (open project) causes this
                        debug!(
                            ?id,
                            tab = self.active_project().unwrap().title,
                            "Setting editor pane",
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
                            debug!(?keybind_pane, ?id, "Removing keybind pane");
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
