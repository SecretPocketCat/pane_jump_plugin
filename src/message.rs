use std::ptr::write_bytes;

use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PipeMessage, PipeSource},
    shim::{open_command_pane_floating, set_timeout, write, write_chars},
};

use crate::{PluginState, WriteQueueItem};

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, strum_macros::AsRefStr, Debug, PartialEq)]
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

                    self.queue_esc();
                    self.set_timer();
                    // todo: handle possible race conditions
                    // maybe just make it a queue of vecs instead?
                    self.queue_write_bytes(vec![
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
                            let lines: Vec<_> = files
                                .lines()
                                .map(|l| l.trim())
                                .filter(|l| !l.is_empty())
                                .collect();

                            if !lines.is_empty() {
                                self.focus_editor_pane();
                                self.queue_esc();

                                for file in lines {
                                    self.queue_write_string(format!(":open {file}"));
                                    self.queue_enter();
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    pub(crate) fn queue_write_string(&mut self, val: String) {
        self.set_timer();
        self.queued_stdin_bytes
            .push_back(WriteQueueItem::String(val));
    }

    pub(crate) fn queue_write_bytes(&mut self, val: Vec<u8>) {
        self.set_timer();
        self.queued_stdin_bytes
            .push_back(WriteQueueItem::Bytes(val));
    }

    pub(crate) fn queue_esc(&mut self) {
        self.queue_write_bytes(vec![27]);
    }

    pub(crate) fn queue_enter(&mut self) {
        self.queue_write_bytes(vec![13]);
    }

    pub(crate) fn set_timer(&mut self) {
        if !self.queue_timer_set {
            self.queue_timer_set = true;
            set_timeout(0.05);
        }
    }

    pub(crate) fn process_timer(&mut self) {
        self.queue_timer_set = false;

        if let Some(item) = self.queued_stdin_bytes.pop_front() {
            match item {
                WriteQueueItem::String(str) => write_chars(&str),
                WriteQueueItem::Bytes(bytes) => zellij_tile::shim::write(bytes),
            }
        }

        if !self.queued_stdin_bytes.is_empty() {
            self.set_timer();
        }
    }
}
