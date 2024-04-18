use itertools::Itertools;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    convert::{TryFrom, TryInto},
};
use zellij_tile::prelude::{CommandToRun, PaneInfo, PipeMessage, PipeSource};

use crate::{
    command_queue::QueuedCommand,
    input::MessageKeybind,
    pane::{DASH_PANE_NAME, FILEPICKER_PANE_NAME, GIT_PANE_NAME},
    wavedash, PluginState, PluginStatus, PLUGIN_NAME,
};

pub(crate) const YAZI_CMD: &str = "yazi --chooser-file /dev/stdout";
pub(crate) const DASH_CMD: &str = "fzf --layout reverse --with-nth 2..";
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

lazy_static! {
    static ref KEYBIND_PANE_MAP: HashMap<&'static str, KeybindPane> = HashMap::from([
        (DASH_PANE_NAME, KeybindPane::WaveDash),
        (FILEPICKER_PANE_NAME, KeybindPane::FilePicker),
        (GIT_PANE_NAME, KeybindPane::Git),
        ("k9s", KeybindPane::K9s),
    ]);
}

impl TryFrom<&PaneInfo> for KeybindPane {
    type Error = ();

    fn try_from(value: &PaneInfo) -> Result<Self, Self::Error> {
        KEYBIND_PANE_MAP
            .get(value.title.as_str())
            .cloned()
            .ok_or(())
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
                            self.command_queue.queue_esc();
                            self.command_queue.queue_write_bytes(vec![
                                // https://sw.kovidgoyal.net/kitty/keyboard-protocol/#legacy-ctrl-mapping-of-ascii-keys
                                2,
                            ]);
                        }
                        MessageKeybind::NewTerminal => {
                            Self::open_floating_pane(None);
                            // todo
                            // self.command_queue.queue_command(QueuedCommand::RenamePane);
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

                            if let Some(new_status) = match keybind_pane {
                                KeybindPane::WaveDash => Some(PluginStatus::Dash {
                                    input: String::default(),
                                }),
                                KeybindPane::FilePicker => Some(PluginStatus::FilePicker),
                                _ => None,
                            } {
                                self.status = new_status;
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
                                self.command_queue.queue_esc();

                                for file in lines {
                                    self.command_queue
                                        .queue_write_string(format!(":open {file}"));
                                    self.command_queue.queue_enter();
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
                                    self.command_queue.queue_command(QueuedCommand::FocusEditor);
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
                    "printf '{opts}' | command cat -n | {DASH_CMD} | awk '{{print $1}}' | zellij pipe --plugin {PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
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
                    "{YAZI_CMD} | zellij pipe --plugin {PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
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
}
