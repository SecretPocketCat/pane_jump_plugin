use itertools::Itertools;
use std::convert::{TryFrom, TryInto};
use zellij_tile::prelude::{CommandToRun, PipeMessage, PipeSource};

use crate::{
    command_queue::{QueuedFocusCommand, QueuedTimerCommand},
    input::MessageKeybind,
    pane::{DASH_PANE_NAME, FILEPICKER_PANE_NAME, GIT_PANE_NAME},
    PluginState, PluginStatus, PLUGIN_NAME,
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
    StatusPaneDash,
    FilePicker,
    Git,
    Terminal,
    K9s,
}

impl KeybindPane {
    fn pane_name(&self) -> &str {
        match self {
            KeybindPane::StatusPaneDash => DASH_PANE_NAME,
            KeybindPane::FilePicker => FILEPICKER_PANE_NAME,
            KeybindPane::Git => GIT_PANE_NAME,
            KeybindPane::Terminal => "term",
            KeybindPane::K9s => "k9s",
        }
    }
}

impl TryFrom<MessageKeybind> for KeybindPane {
    type Error = ();

    fn try_from(value: MessageKeybind) -> Result<Self, Self::Error> {
        match value {
            MessageKeybind::Wavedash => Ok(KeybindPane::StatusPaneDash),
            MessageKeybind::FilePicker => Ok(KeybindPane::FilePicker),
            MessageKeybind::Git => Ok(KeybindPane::Git),
            MessageKeybind::Terminal => Ok(KeybindPane::Terminal),
            MessageKeybind::K9s => Ok(KeybindPane::K9s),
            MessageKeybind::FocusEditorPane
            | MessageKeybind::HxBufferJumplist
            | MessageKeybind::HxOpenFile
            | MessageKeybind::NewTerminal => Err(()),
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
                        MessageKeybind::HxOpenFile => {
                            self.focus_editor_pane();
                            self.command_queue.queue_esc();
                            self.command_queue.queue_write_bytes(vec![32]); // SPC
                            self.command_queue.queue_write_bytes(vec![102]); // f
                        }
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
                            self.spawned_extra_term_count += 1;
                            self.command_queue
                                .queue_focus_command(QueuedFocusCommand::RenamePane(format!(
                                    "Terminal #{}",
                                    self.spawned_extra_term_count
                                )));
                        }
                        MessageKeybind::Wavedash
                        | MessageKeybind::FilePicker
                        | MessageKeybind::Terminal
                        | MessageKeybind::Git
                        | MessageKeybind::K9s => {
                            let keybind_pane: KeybindPane = keybind.try_into().unwrap();
                            if let Some(pane_id) = self.keybind_panes.get(&keybind_pane) {
                                eprintln!(
                                    "Focusing keybind pane '{:?}', id: '{:?}'",
                                    keybind_pane, pane_id
                                );
                                pane_id.focus();
                            } else {
                                eprintln!("Opening keybind pane '{:?}'", keybind_pane);

                                Self::open_floating_pane(self.spawn_pane_command(&keybind_pane));
                                self.command_queue.queue_focus_command(
                                    QueuedFocusCommand::MarkKeybindPane(keybind_pane),
                                );
                                self.command_queue.queue_focus_command(
                                    QueuedFocusCommand::RenamePane(
                                        keybind_pane.pane_name().to_string(),
                                    ),
                                );
                            }

                            if let Some(new_status) = match keybind_pane {
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

    fn spawn_pane_command(&self, keybind_pane: &KeybindPane) -> Option<CommandToRun> {
        match keybind_pane {
            KeybindPane::Git => Some(CommandToRun::new("lazygit")),
            KeybindPane::K9s => Some(CommandToRun::new("k9s")),
            KeybindPane::Terminal => None,
            KeybindPane::StatusPaneDash => {
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
