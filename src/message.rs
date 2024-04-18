use itertools::Itertools;
use std::convert::{TryFrom, TryInto};
use zellij_tile::{
    prelude::{CommandToRun, PaneInfo, PipeMessage, PipeSource},
    shim::{set_timeout, write_chars},
};

use crate::{input::MessageKeybind, pane::GIT_PANE_NAME, PluginState, WriteQueueItem, PLUGIN_NAME};

pub(crate) const MSG_CLIENT_ID_ARG: &str = "picker_id";

#[derive(strum_macros::EnumString, strum_macros::AsRefStr, Debug, PartialEq)]
pub(crate) enum MessageType {
    OpenFile,
    FocusPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum KeybindPane {
    WaveDash,
    FilePicker,
    Git,
    Terminal,
    K9s,
}

impl TryFrom<MessageKeybind> for KeybindPane {
    type Error = ();

    fn try_from(value: MessageKeybind) -> Result<Self, Self::Error> {
        match value {
            MessageKeybind::Wavedash => Ok(KeybindPane::WaveDash),
            MessageKeybind::FilePicker => Ok(KeybindPane::FilePicker),
            MessageKeybind::Git => Ok(KeybindPane::Git),
            MessageKeybind::Terminal => Ok(KeybindPane::Terminal),
            MessageKeybind::K9s => Ok(KeybindPane::K9s),
            MessageKeybind::FocusEditorPane
            | MessageKeybind::HxBufferJumplist
            | MessageKeybind::NewTerminal => Err(()),
        }
    }
}

impl TryFrom<&PaneInfo> for KeybindPane {
    type Error = ();

    fn try_from(value: &PaneInfo) -> Result<Self, Self::Error> {
        if value.title.contains("| fzf") {
            return Ok(KeybindPane::WaveDash);
        }

        match value.title.as_str() {
            GIT_PANE_NAME => Ok(KeybindPane::Git),
            "k9s" => Ok(KeybindPane::K9s),
            _ => Err(()),
        }
    }
}

impl PluginState {
    pub(crate) fn handle_pipe_message(&mut self, pipe_message: PipeMessage) -> bool {
        if pipe_message.source == PipeSource::Keybind {
            match pipe_message.name.parse::<MessageKeybind>() {
                Ok(keybind) => {
                    match keybind {
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
                            Self::open_floating_pane(None);
                        }
                        MessageKeybind::Wavedash
                        | MessageKeybind::FilePicker
                        | MessageKeybind::Terminal
                        | MessageKeybind::Git
                        | MessageKeybind::K9s => {
                            let keybind_pane: KeybindPane = keybind.try_into().unwrap();
                            if let Some(pane_id) = self.keybind_panes.get(&keybind_pane) {
                                pane_id.focus();
                            } else {
                                Self::open_floating_pane(self.spawn_pane_command(&keybind_pane));
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
                                self.queue_esc();

                                for file in lines {
                                    self.queue_write_string(format!(":open {file}"));
                                    self.queue_enter();
                                }
                            }
                        }
                        MessageType::FocusPane => {
                            if let Some(idx) = payload
                                .lines()
                                .next()
                                .map(|l| l.parse::<usize>().ok())
                                .flatten()
                            {
                                if let Some(pane) = self.dash_panes.get(idx - 1) {
                                    pane.id.focus();
                                    // todo: need to queue that instead?
                                    // self.focus_editor_pane();
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    fn spawn_pane_command(&self, keybind_pane: &KeybindPane) -> Option<CommandToRun> {
        match keybind_pane {
            KeybindPane::Git => Some(CommandToRun::new("lazygit")),
            KeybindPane::K9s => Some(CommandToRun::new("k9s")),
            KeybindPane::Terminal => None,
            KeybindPane::WaveDash => {
                let opts = self.dash_panes.iter().map(|p| &p.title).join("\n");
                let cmd = format!(
                    "printf '{opts}' | command cat -n | fzf --layout reverse --with-nth 2.. | awk '{{print $1}}' | zellij pipe --plugin {PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
                    MessageType::FocusPane.as_ref(),
                    self.msg_client_id
                );
                Some(CommandToRun {
                    // path: "fish".into(),
                    path: "bash".into(),
                    args: vec!["-c".to_string(), cmd],
                    cwd: None,
                })
            }
            KeybindPane::FilePicker => {
                let cmd = format!(
                    "yazi --chooser-file /dev/stdout | zellij pipe --plugin {PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
                    MessageType::OpenFile.as_ref(),
                    self.msg_client_id
                );
                Some(CommandToRun {
                    path: "bash".into(),
                    args: vec!["-c".to_string(), cmd],
                    cwd: None,
                })
            }
        }
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
