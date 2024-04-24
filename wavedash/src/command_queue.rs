use std::collections::VecDeque;
use tracing::{instrument, warn};
use utils::{fzf::get_fzf_pane_cmd, pane::PaneFocus};
use zellij_tile::shim::{set_timeout, write_chars};

use crate::{input::KeybindPane, message::MessageType, PluginState, PLUGIN_NAME};

pub(crate) enum QueuedTimerCommand {
    WriteString(String),
    WriteBytes(Vec<u8>),
    FocusEditor,
    #[allow(dead_code)]
    ExtraDelay(f64),
    ProcessQueuedTabUpdate,
}

#[allow(clippy::enum_variant_names)]
pub(crate) enum QueuedFocusCommand {
    RenamePane(String),
    TriggerRenameInput,
    MarkKeybindPane(KeybindPane),
    MarkTerminalPane(String),
}

#[derive(Default)]
pub(crate) struct CommandQueue {
    timer_queue: VecDeque<QueuedTimerCommand>,
    focus_queue: VecDeque<QueuedFocusCommand>,
    timer_set: bool,
}

impl CommandQueue {
    pub(crate) fn queue_timer_command(&mut self, queued_command: QueuedTimerCommand) {
        self.set_timer(0.);
        self.timer_queue.push_back(queued_command);
    }

    pub(crate) fn queue_write_string(&mut self, val: String) {
        self.queue_timer_command(QueuedTimerCommand::WriteString(val));
    }

    pub(crate) fn queue_write_bytes(&mut self, val: Vec<u8>) {
        self.queue_timer_command(QueuedTimerCommand::WriteBytes(val));
    }

    pub(crate) fn queue_esc(&mut self) {
        self.queue_write_bytes(vec![27]);
    }

    pub(crate) fn queue_enter(&mut self) {
        self.queue_write_bytes(vec![13]);
    }

    pub(crate) fn queue_focus_command(&mut self, queued_command: QueuedFocusCommand) {
        self.focus_queue.push_back(queued_command);
    }

    pub(crate) fn set_timer(&mut self, extra_delay: f64) {
        if !self.timer_set {
            self.timer_set = true;
            set_timeout(0.03 + extra_delay);
        }
    }

    fn dequeue_timer_command(&mut self) -> Option<QueuedTimerCommand> {
        self.timer_set = false;
        let res = self.timer_queue.pop_front();
        if !self.timer_queue.is_empty() {
            if let Some(QueuedTimerCommand::ExtraDelay(extra)) = res {
                self.set_timer(extra);
            } else {
                self.set_timer(0.);
            }
        }
        res
    }

    pub(crate) fn dequeue_focus_command(&mut self) -> Option<QueuedFocusCommand> {
        self.focus_queue.pop_front()
    }
}

impl PluginState {
    pub(crate) fn handle_timer(&mut self) {
        if let Some(item) = self.command_queue.dequeue_timer_command() {
            match item {
                QueuedTimerCommand::WriteString(str) => write_chars(&str),
                QueuedTimerCommand::WriteBytes(bytes) => zellij_tile::shim::write(bytes),
                QueuedTimerCommand::FocusEditor => self.focus_editor_pane(),
                QueuedTimerCommand::ExtraDelay(_) => {}
                QueuedTimerCommand::ProcessQueuedTabUpdate => self.handle_queued_tab_update(),
            }
        }
    }

    #[instrument(skip(self))]
    pub(crate) fn handle_command_result(
        &mut self,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    ) {
        if exit_code.is_some_and(|c| c != 0) {
            warn!(
                code=exit_code.unwrap(),
                stderr=?String::from_utf8_lossy(&stderr),
                "Command has failed",
            );
            return;
        }

        // todo: insert keybind pane etc.
        Self::open_floating_pane(Some(get_fzf_pane_cmd(
            String::from_utf8_lossy(&stdout).lines(),
            PLUGIN_NAME,
            MessageType::OpenProject.as_ref(),
            self.msg_client_id,
            false,
        )));
    }

    pub(crate) fn handle_focus_change(&mut self, focus: PaneFocus) {
        if self.project_uninit() {
            panic!("Attempted to processe focus queue when project is not initialized");
        }

        let id = focus.id();
        while let Some(item) = self.command_queue.dequeue_focus_command() {
            match item {
                QueuedFocusCommand::MarkKeybindPane(keybind_pane) => {
                    self.active_project_mut()
                        .unwrap()
                        .keybind_panes
                        .entry(keybind_pane)
                        .or_insert(id);
                }
                QueuedFocusCommand::RenamePane(new_name) => {
                    id.rename(&new_name);
                }
                QueuedFocusCommand::MarkTerminalPane(title) => {
                    self.active_project_mut()
                        .unwrap()
                        .terminal_panes
                        .entry(id)
                        .or_insert(title);
                }
                QueuedFocusCommand::TriggerRenameInput => {
                    // self.command_queue
                    //     .queue_timer_command(QueuedTimerCommand::ExtraDelay(3.));
                    // todo: alt+p
                    // self.command_queue.queue_write_bytes(vec![0x1b, 112]);
                    // self.command_queue.queue_write_string("r".to_string());
                }
            }
        }
    }
}
