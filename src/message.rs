use zellij_tile::{
    prelude::{CommandToRun, PipeMessage, PipeSource},
    shim::open_command_pane,
};

use crate::PluginState;

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
}

impl PluginState {
    pub(crate) fn handle_pipe_message(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.source == PipeSource::Keybind {
            self.open_picker();
        } else if pipe_message
            .args
            .get(MSG_CLIENT_ID_ARG)
            .is_some_and(|guid| guid == &self.msg_client_id.to_string())
        {
            if let Ok(msg_type) = pipe_message.name.parse::<MessageType>() {
                match msg_type {
                    MessageType::OpenFile => {
                        if let Some(files) = pipe_message.payload {
                            for file in files.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                                open_command_pane(CommandToRun {
                                    path: "hx".into(),
                                    args: vec![file.to_string()],
                                    cwd: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        false
    }
}
