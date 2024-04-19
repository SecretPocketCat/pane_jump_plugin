use crate::{command_queue::QueuedTimerCommand, PluginState};

use zellij_tile::prelude::{PipeMessage, PipeSource};

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, strum_macros::AsRefStr, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
    FocusPane,
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
                        MessageType::FocusPane => {
                            if let Some(idx) =
                                payload.lines().next().and_then(|l| l.parse::<usize>().ok())
                            {
                                if let Some(pane) = self.dash_panes.get(idx - 1) {
                                    pane.id.focus();
                                    self.command_queue
                                        .queue_timer_command(QueuedTimerCommand::FocusEditor);
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }
}
