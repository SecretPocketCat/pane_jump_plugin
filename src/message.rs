use std::convert::{TryFrom, TryInto};

use zellij_tile::{
    prelude::{CommandToRun, FloatingPaneCoordinates, PaneInfo, PipeMessage, PipeSource},
    shim::{
        get_plugin_ids, open_command_pane_floating, open_terminal_floating, set_timeout,
        write_chars,
    },
};

use crate::{input::MessageKeybind, pane::GIT_PANE_NAME, PluginState, WriteQueueItem};

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, strum_macros::AsRefStr, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeybindPane {
    Git,
    Terminal,
    K9s,
}

impl TryFrom<MessageKeybind> for KeybindPane {
    type Error = ();

    fn try_from(value: MessageKeybind) -> Result<Self, Self::Error> {
        match value {
            MessageKeybind::Git => Ok(KeybindPane::Git),
            MessageKeybind::Terminal => Ok(KeybindPane::Terminal),
            MessageKeybind::K9s => Ok(KeybindPane::K9s),
            _ => Err(()),
        }
    }
}

impl TryFrom<&PaneInfo> for KeybindPane {
    type Error = ();

    fn try_from(value: &PaneInfo) -> Result<Self, Self::Error> {
        match value.title.as_str() {
            GIT_PANE_NAME => Ok(KeybindPane::Git),
            "k9s" => Ok(KeybindPane::K9s),
            _ => Err(()),
        }
    }
}

impl KeybindPane {
    fn spawn_pane_command(&self) -> Option<CommandToRun> {
        match self {
            KeybindPane::Git => Some(CommandToRun::new("lazygit")),
            KeybindPane::K9s => Some(CommandToRun::new("k9s")),
            KeybindPane::Terminal => None,
        }
    }
}

impl PluginState {
    pub(crate) fn handle_pipe_message(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.source == PipeSource::Keybind {
            match pipe_message.name.parse::<MessageKeybind>() {
                Ok(keybind) => {
                    match keybind {
                        MessageKeybind::FilePicker => self.open_picker(),
                        MessageKeybind::FocusEditorPane => self.editor_pane_id.focus(),
                        MessageKeybind::HxBufferJumplist => {
                            self.focus_editor_pane();
                            self.queue_esc();
                            self.set_timer();
                            self.queue_write_bytes(vec![
                                // https://sw.kovidgoyal.net/kitty/keyboard-protocol/#legacy-ctrl-mapping-of-ascii-keys
                                2,
                            ]);
                        }
                        MessageKeybind::NewTerminal => {
                            // todo
                        }
                        MessageKeybind::Terminal | MessageKeybind::Git | MessageKeybind::K9s => {
                            let keybind_pane: KeybindPane = keybind.try_into().unwrap();
                            if let Some(pane_id) = self.keybind_panes.get(&keybind_pane) {
                                pane_id.focus();
                            } else {
                                Self::open_floating_pane(keybind_pane.spawn_pane_command());
                            }
                        }
                    }
                }
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
