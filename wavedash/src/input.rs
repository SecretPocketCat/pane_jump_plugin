use crate::{command_queue::QueuedFocusCommand, message::MessageType, PluginState};
use std::convert::{TryFrom, TryInto};
use tracing::{debug, error, instrument};
use utils::{
    fzf::{fzf_pane_cmd, run_find_repos_command},
    message::MSG_CLIENT_ID_ARG,
    DASH_PLUGIN_NAME,
};
use zellij_tile::prelude::{CommandToRun, PipeMessage};

pub(crate) const YAZI_CMD: &str = "yazi --chooser-file /dev/stdout";

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageKeybind {
    OpenProject,
    DashProject,
    DashStatus,
    DashTerminal,
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
    OpenProject,
    ProjectDash,
    TerminalPaneDash,
    StatusPaneDash,
    FilePicker,
    Git,
    Terminal,
    K9s,
}

impl KeybindPane {
    fn pane_name(&self) -> &str {
        match self {
            KeybindPane::OpenProject => "open_project",
            KeybindPane::ProjectDash => "dash_project",
            KeybindPane::StatusPaneDash => "dash_status",
            KeybindPane::TerminalPaneDash => "dash_terminal",
            KeybindPane::FilePicker => "filepicker",
            KeybindPane::Git => "git",
            KeybindPane::Terminal => "term",
            KeybindPane::K9s => "k9s",
        }
    }
}

impl TryFrom<MessageKeybind> for KeybindPane {
    type Error = ();

    fn try_from(value: MessageKeybind) -> Result<Self, Self::Error> {
        match value {
            MessageKeybind::OpenProject => Ok(KeybindPane::OpenProject),
            MessageKeybind::DashProject => Ok(KeybindPane::ProjectDash),
            MessageKeybind::DashStatus => Ok(KeybindPane::StatusPaneDash),
            MessageKeybind::DashTerminal => Ok(KeybindPane::TerminalPaneDash),
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
    #[instrument(skip_all)]
    pub(crate) fn handle_keybind_message(&mut self, pipe_message: PipeMessage) {
        match pipe_message.name.parse::<MessageKeybind>() {
            Ok(keybind) => {
                match keybind {
                    MessageKeybind::OpenProject => {
                        run_find_repos_command(
                            &*self
                                .root_config
                                .as_ref()
                                .unwrap()
                                .root_path
                                .to_string_lossy(),
                        );
                    }
                    MessageKeybind::FocusEditorPane => self.focus_editor_pane(),
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
                        let proj = self.active_project_mut().unwrap();
                        Self::open_floating_pane(None);
                        proj.spawned_extra_term_count += 1;
                        let title = format!("Terminal #{}", proj.spawned_extra_term_count);
                        self.command_queue.queue_focus_command(
                            QueuedFocusCommand::MarkTerminalPane(title.clone()),
                        );
                        self.command_queue
                            .queue_focus_command(QueuedFocusCommand::RenamePane("".to_string()));
                        self.command_queue
                            .queue_focus_command(QueuedFocusCommand::TriggerRenameInput);
                    }
                    MessageKeybind::DashProject
                    | MessageKeybind::DashStatus
                    | MessageKeybind::DashTerminal
                    | MessageKeybind::FilePicker
                    | MessageKeybind::Terminal
                    | MessageKeybind::Git
                    | MessageKeybind::K9s => {
                        let keybind_pane: KeybindPane = keybind.try_into().unwrap();
                        debug!(?keybind_pane, "Triggered keybindpane");
                        if let Some(pane_id) = self
                            .active_project()
                            .unwrap()
                            .keybind_panes
                            .get(&keybind_pane)
                        {
                            pane_id.focus();
                        } else {
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
                error!(?pipe_message, "Unknown keybind pipe name");
            }
        }
    }

    fn spawn_pane_command(&self, keybind_pane: &KeybindPane) -> Option<CommandToRun> {
        match keybind_pane {
            KeybindPane::Git => Some(CommandToRun::new("lazygit")),
            KeybindPane::K9s => Some(CommandToRun::new("k9s")),
            KeybindPane::Terminal => None,
            KeybindPane::OpenProject => None,
            KeybindPane::ProjectDash => Some(fzf_pane_cmd(
                self.projects.values().map(|p| p.title.as_str()),
                MessageType::FocusProject.as_ref(),
                self.msg_client_id,
                false,
            )),
            KeybindPane::StatusPaneDash => Some(fzf_pane_cmd(
                self.active_project()
                    .unwrap()
                    .status_panes
                    .values()
                    .map(String::as_str),
                MessageType::FocusStatusPane.as_ref(),
                self.msg_client_id,
                true,
            )),
            KeybindPane::TerminalPaneDash => Some(fzf_pane_cmd(
                self.active_project()
                    .unwrap()
                    .terminal_panes
                    .values()
                    .map(String::as_str),
                MessageType::FocusTerminalPane.as_ref(),
                self.msg_client_id,
                true,
            )),
            KeybindPane::FilePicker => {
                let cmd = format!(
                    "{YAZI_CMD} | zellij pipe --plugin {DASH_PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
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
