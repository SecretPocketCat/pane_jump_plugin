use std::collections::BTreeMap;
use utils::{
    fzf::{get_fzf_pane_cmd, run_find_repos_command},
    message::MSG_CLIENT_ID_ARG,
    pane::PaneId,
    template::wavedash_template,
};
use uuid::Uuid;
use zellij_tile::prelude::*;

const PLUGIN_NAME: &str = "project_picker";

#[derive(Default)]
enum PluginStatus {
    #[default]
    Init,
    Picking,
    Picked,
}

struct PluginState {
    status: PluginStatus,
    pane_id: PaneId,
    msg_client_id: Uuid,
    projects: Vec<String>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: Default::default(),
            pane_id: PaneId::Terminal(0),
            msg_client_id: Uuid::new_v4(),
            projects: Default::default(),
        }
    }
}

impl PluginState {
    fn show_project_selection(&self) {
        open_command_pane_in_place(get_fzf_pane_cmd(
            self.projects.iter().map(String::as_str),
            PLUGIN_NAME,
            "pick_project",
            self.msg_client_id,
            false,
        ));
    }
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        show_self(true);
        self.pane_id = PaneId::Plugin(get_plugin_ids().plugin_id);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::RunCommandResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                if let PluginStatus::Picked = self.status {
                    if tabs.len() > 1 {
                        // last pane, so this will close the tab
                        close_self();
                    }
                }
            }
            Event::PaneUpdate(PaneManifest { panes }) => match self.status {
                PluginStatus::Init => {
                    run_find_repos_command(&*get_plugin_ids().initial_cwd.to_string_lossy());
                    self.status = PluginStatus::Picking;
                }
                PluginStatus::Picking => {
                    if let Some(pane) = panes
                        .values()
                        .flatten()
                        .find(|p| p.terminal_command.is_some() && p.title != PLUGIN_NAME)
                    {
                        let id = PaneId::from(pane);
                        id.rename(PLUGIN_NAME);
                    }
                }
                PluginStatus::Picked => {}
            },
            Event::RunCommandResult(exit_code, stdout, stderr, _ctx) => {
                if let PluginStatus::Picking = self.status {
                    if exit_code.is_some_and(|c| c != 0) {
                        eprintln!(
                            "Command has failed - exit code: '{}', err: {}",
                            exit_code.unwrap(),
                            String::from_utf8_lossy(&stderr)
                        );
                    } else {
                        self.projects = String::from_utf8_lossy(&stdout)
                            .lines()
                            .map(Into::into)
                            .collect();
                        self.show_project_selection();
                    }
                }
            }
            _ => unimplemented!("{event:?}"),
        }

        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message
            .args
            .get(MSG_CLIENT_ID_ARG)
            .is_some_and(|guid| guid == &self.msg_client_id.to_string())
        {
            if let Some(cwd) = pipe_message
                .payload
                .and_then(|p| p.lines().next().map(|l| l.to_string()))
            {
                // todo: the name should be the cwd without the workspace root
                // have to impl to figure out the code to get the workspace path
                let name = cwd.replace(
                    &get_plugin_ids().initial_cwd.to_string_lossy().to_string(),
                    "",
                );
                // close the in-place fzf pane
                close_focus();
                new_tabs_with_layout(&wavedash_template(&cwd, &name, true));
                self.status = PluginStatus::Picked;
            } else {
                // replace cancelled fzf with a new one
                close_focus();
                self.show_project_selection();
            }
        }

        false
    }
}
