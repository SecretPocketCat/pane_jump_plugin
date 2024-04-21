use std::collections::BTreeMap;
use utils::{get_fzf_pane_cmd, run_find_repos_command};
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

#[derive(Default)]
struct PluginState {
    status: PluginStatus,
    msg_client_id: Uuid,
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.msg_client_id = Uuid::new_v4();
        show_self(true);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::PaneUpdate,
            // EventType::TabUpdate,
            EventType::RunCommandResult,
            // EventType::CustomMessage,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            // Event::TabUpdate(tabs) => self.handle_tab_update(&tabs),
            Event::PaneUpdate(panes) => match self.status {
                PluginStatus::Init => {
                    run_find_repos_command(&*get_plugin_ids().initial_cwd.to_string_lossy());
                    self.status = PluginStatus::Picking;
                    // open_command_pane_in_place(get_fzf_pane_cmd(, , , ))
                    // todo: open picker
                }
                PluginStatus::Picking => {
                    // todo: restart if picker was escaped without picking an option
                }
                PluginStatus::Picked => {}
            },
            Event::RunCommandResult(exit_code, stdout, stderr, _ctx) => {
                // open_command_pane_in_place(get_fzf_pane_cmd(, , , ))

                if exit_code.is_some_and(|c| c != 0) {
                    eprintln!(
                        "Command has failed - exit code: '{}', err: {}",
                        exit_code.unwrap(),
                        String::from_utf8_lossy(&stderr)
                    );
                } else {
                    open_command_pane_in_place(get_fzf_pane_cmd(
                        String::from_utf8_lossy(&stdout).lines(),
                        PLUGIN_NAME,
                        "pick_project",
                        self.msg_client_id,
                    ));
                }
            }
            // Event::CustomMessage(message, payload) => {
            //     // if message == "session_layout" {
            //     //     self.set_new_tab_layout(Self::format_layout(payload));
            //     // }
            // }
            _ => unimplemented!("{event:?}"),
        }

        false
    }

    // fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
    //     self.handle_pipe_message(pipe_message)
    // }
}
