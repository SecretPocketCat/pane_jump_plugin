use std::ptr::write_bytes;

use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PipeMessage, PipeSource},
    shim::{open_command_pane_floating, set_timeout, write, write_chars},
};

use crate::PluginState;

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
}

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageKeybind {
    FilePicker,
    FocusEditorPane,
    HxBufferJumplist,
    Git,
}

impl PluginState {
    pub(crate) fn handle_pipe_message(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.source == PipeSource::Keybind {
            match pipe_message.name.parse::<MessageKeybind>() {
                Ok(MessageKeybind::FilePicker) => self.open_picker(),
                Ok(MessageKeybind::FocusEditorPane) => self.editor_pane_id.focus(),
                Ok(MessageKeybind::HxBufferJumplist) => {
                    self.focus_editor_pane();
                    // CSI seq: ESC [
                    // then key????
                    // then modifiers

                    zellij_tile::shim::write(vec![27]); // send ESC
                    set_timeout(0.05);
                    // todo: handle possible race conditions
                    // maybe just make it a queue of vecs instead?
                    self.queued_stdin_bytes = Some(vec![
                        // https://sw.kovidgoyal.net/kitty/keyboard-protocol/#legacy-ctrl-mapping-of-ascii-keys
                        2,
                    ]);

                    // zellij_tile::shim::write(vec![
                    // legacy CTRL maping - C0 code
                    // 0x1b, 0x5b, // CSI seq
                    // 98,   // b
                    // // 0x3b,  // delimiter? ';'
                    // 5,    // modifiers - ctrl
                    // 0x75, // termination 'u'
                    // ]);
                }
                Ok(MessageKeybind::Git) => match self.git_pane_id {
                    Some(id) => id.focus(),
                    None => {
                        open_command_pane_floating(
                            CommandToRun {
                                path: "lazygit".into(),
                                args: vec![],
                                cwd: None,
                            },
                            Some(
                                FloatingPaneCoordinates::default()
                                    .with_x_fixed(0)
                                    .with_y_fixed(0)
                                    .with_width_percent(95)
                                    .with_height_percent(90),
                            ),
                        );
                    }
                },
                Err(_) => {
                    eprintln!("Missing name for keybind pipe message {pipe_message:?}");
                }
            }
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
                                write_chars(&format!(":open {file}"));

                                // open_command_pane(CommandToRun {
                                //     path: "hx".into(),
                                //     args: vec![file.to_string()],
                                //     cwd: None,
                                // });
                            }
                        }
                    }
                }
            }
        }

        false
    }
}
