use std::collections::VecDeque;

use zellij_tile::shim::{set_timeout, write_chars};

use crate::PluginState;

pub(crate) enum QueuedCommand {
    WriteString(String),
    WriteBytes(Vec<u8>),
    FocusEditor,
}

#[derive(Default)]
pub(crate) struct CommandQueue {
    queue: VecDeque<QueuedCommand>,
    timer_set: bool,
}

impl CommandQueue {
    pub(crate) fn queue_command(&mut self, queued_command: QueuedCommand) {
        self.set_timer();
        self.queue.push_back(queued_command);
    }

    pub(crate) fn queue_write_string(&mut self, val: String) {
        self.queue_command(QueuedCommand::WriteString(val));
    }

    pub(crate) fn queue_write_bytes(&mut self, val: Vec<u8>) {
        self.queue_command(QueuedCommand::WriteBytes(val));
    }

    pub(crate) fn queue_esc(&mut self) {
        self.queue_write_bytes(vec![27]);
    }

    pub(crate) fn queue_enter(&mut self) {
        self.queue_write_bytes(vec![13]);
    }

    pub(crate) fn set_timer(&mut self) {
        if !self.timer_set {
            self.timer_set = true;
            set_timeout(0.05);
        }
    }

    pub(crate) fn dequeue(&mut self) -> Option<QueuedCommand> {
        self.timer_set = false;
        let res = self.queue.pop_front();
        if !self.queue.is_empty() {
            self.set_timer();
        }
        res
    }
}

impl PluginState {
    pub(crate) fn process_timer(&mut self) {
        if let Some(item) = self.command_queue.dequeue() {
            match item {
                QueuedCommand::WriteString(str) => write_chars(&str),
                QueuedCommand::WriteBytes(bytes) => zellij_tile::shim::write(bytes),
                QueuedCommand::FocusEditor => self.focus_editor_pane(),
            }
        }
    }
}
