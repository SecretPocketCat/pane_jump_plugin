use crate::{command_queue::QueuedTimerCommand, PluginState};

use utils::{
    fzf::parse_fzf_index, message::MSG_CLIENT_ID_ARG, project::PROJECT_ROOT_RESP_MESSAGE_NAME,
    template::wavedash_template,
};
use zellij_tile::{
    prelude::{PipeMessage, PipeSource},
    shim::{close_focus, focus_or_create_tab, new_tabs_with_layout},
};

#[derive(strum_macros::EnumString, strum_macros::AsRefStr, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
    OpenProject,
    FocusProject,
    FocusStatusPane,
    FocusTerminalPane,
}

impl PluginState {
    pub(crate) fn handle_pipe_message(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.source == PipeSource::Keybind {
            self.handle_keybind_message(pipe_message);
        } else if pipe_message
            .args
            .get(MSG_CLIENT_ID_ARG)
            .is_some_and(|guid| guid == &self.msg_client_id.to_string())
        {
            if let Some(payload) = pipe_message.payload {
                if let Ok(msg_type) = pipe_message.name.parse::<MessageType>() {
                    match msg_type {
                        MessageType::OpenFile => {
                            let lines: Vec<_> = payload
                                .lines()
                                .map(|l| l.trim())
                                .filter(|l| !l.is_empty())
                                .collect();

                            if !lines.is_empty() {
                                self.focus_editor_pane();
                                self.command_queue.queue_esc();

                                for file in lines {
                                    self.command_queue
                                        .queue_write_string(format!(":open {file}"));
                                    self.command_queue.queue_enter();
                                }
                            }
                        }
                        MessageType::OpenProject => {
                            if let Some(option) = parse_fzf_index::<usize>(&payload)
                                .and_then(|i| self.project_options.get(i))
                            {
                                if self.projects.contains_key(&option.title) {
                                    focus_or_create_tab(&option.title);
                                } else {
                                    new_tabs_with_layout(&wavedash_template(&option, false));
                                }
                            }

                            // close fzf pane
                            close_focus();
                        }
                        MessageType::FocusProject => {
                            if let Some(tab_title) = payload.lines().next() {
                                if self.projects.contains_key(tab_title) {
                                    focus_or_create_tab(tab_title);
                                }
                            }
                        }
                        MessageType::FocusStatusPane => {
                            if let Some(idx) = parse_fzf_index::<usize>(&payload) {
                                if let Some((id, _)) =
                                    self.active_project().unwrap().status_panes.get_index(idx)
                                {
                                    id.focus();
                                    self.command_queue
                                        .queue_timer_command(QueuedTimerCommand::FocusEditor);
                                }
                            }
                        }
                        MessageType::FocusTerminalPane => {
                            if let Some(idx) = parse_fzf_index::<usize>(&payload) {
                                if let Some((id, _)) =
                                    self.active_project().unwrap().terminal_panes.get_index(idx)
                                {
                                    id.focus();
                                }
                            }
                        }
                    }
                }
            }
        } else if pipe_message.name == PROJECT_ROOT_RESP_MESSAGE_NAME {
            if let Some(conf) = pipe_message.payload {
                self.root_config = Some(
                    serde_json::from_str(&conf).expect("Failed to deserialize project root config"),
                );
            }
        }

        false
    }
}
