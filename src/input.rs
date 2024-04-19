use crate::{
    command_queue::QueuedFocusCommand,
    message::{MessageType, MSG_CLIENT_ID_ARG},
    pane::{DASH_PANE_NAME, FILEPICKER_PANE_NAME, GIT_PANE_NAME},
    PluginState, PLUGIN_NAME,
};

use itertools::Itertools;
use std::convert::{TryFrom, TryInto};
use zellij_tile::prelude::{CommandToRun, PipeMessage};

pub(crate) const YAZI_CMD: &str = "yazi --chooser-file /dev/stdout";
pub(crate) const DASH_CMD: &str = "fzf --layout reverse --with-nth 2..";

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageKeybind {
    Wavedash,
    FilePicker,
    FocusEditorPane,
    HxBufferJumplist,
    HxOpenFile,
    Git,
    Terminal,
    NewTerminal,
    K9s,
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
    pub(crate) fn handle_keybind_message(&mut self, pipe_message: PipeMessage) {
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
                            self.command_queue
                                .queue_focus_command(QueuedFocusCommand::RenamePane(
                                    keybind_pane.pane_name().to_string(),
                                ));
                        }
                    }
                }
            }
            Err(_) => {
                eprintln!("Missing name for keybind pipe message {pipe_message:?}");
            }
        }
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
