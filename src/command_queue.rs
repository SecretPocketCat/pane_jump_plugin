use std::collections::VecDeque;

use zellij_tile::shim::{set_timeout, write_chars};

use crate::{input::KeybindPane, pane::PaneFocus, PluginState};

pub(crate) enum QueuedTimerCommand {
    WriteString(String),
    WriteBytes(Vec<u8>),
    FocusEditor,
}

#[allow(clippy::enum_variant_names)]
pub(crate) enum QueuedFocusCommand {
    RenamePane(String),
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
        self.set_timer();
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

    pub(crate) fn set_timer(&mut self) {
        if !self.timer_set {
            self.timer_set = true;
            set_timeout(0.05);
        }
    }

    pub(crate) fn dequeue_timer_command(&mut self) -> Option<QueuedTimerCommand> {
        self.timer_set = false;
        let res = self.timer_queue.pop_front();
        if !self.timer_queue.is_empty() {
            self.set_timer();
        }
        res
    }

    pub(crate) fn dequeue_focus_command(&mut self) -> Option<QueuedFocusCommand> {
        self.focus_queue.pop_front()
    }
}

impl PluginState {
    pub(crate) fn process_timer(&mut self) {
        if let Some(item) = self.command_queue.dequeue_timer_command() {
            match item {
                QueuedTimerCommand::WriteString(str) => write_chars(&str),
                QueuedTimerCommand::WriteBytes(bytes) => zellij_tile::shim::write(bytes),
                QueuedTimerCommand::FocusEditor => self.focus_editor_pane(),
            }
        }
    }

    pub(crate) fn process_focus_change(&mut self, focus: PaneFocus) {
        let id = focus.id();
        while let Some(item) = self.command_queue.dequeue_focus_command() {
            match item {
                QueuedFocusCommand::MarkKeybindPane(keybind_pane) => {
                    eprintln!("Marking keybind pane '{keybind_pane:?}', id: '{id:?}'");
                    self.keybind_panes.entry(keybind_pane).or_insert(id);
                }
                QueuedFocusCommand::RenamePane(new_name) => {
                    id.rename(&new_name);
                }
                QueuedFocusCommand::MarkTerminalPane(title) => {
                    self.terminal_panes.entry(id).or_insert(title);
                }
            }
        }
    }
}
