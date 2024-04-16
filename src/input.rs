use zellij_tile::prelude::Key;

use crate::{file_picker::PickerStatus, PluginState, PluginStatus};

#[derive(strum_macros::EnumString, Debug, PartialEq)]
pub(crate) enum MessageKeybind {
    FilePicker,
    FocusEditorPane,
    HxBufferJumplist,
    Git,
    Terminal,
    NewTerminal,
    K9s,
}

impl PluginState {
    pub(crate) fn handle_key(&mut self, key: Key) {
        match &self.status {
            PluginStatus::FilePicker(status) => self.handle_filepicker_key(key, status.clone()),
            PluginStatus::Editor => self.handle_dash_key(key),
            _ => {}
        }
    }

    fn handle_filepicker_key(&mut self, key: Key, picker_status: PickerStatus) {
        match key {
            Key::Esc => {
                if let PickerStatus::Picking(id) = picker_status {
                    id.close();
                    self.status = PluginStatus::Editor;
                }
            }
            _ => {}
        }
    }

    fn handle_dash_key(&mut self, key: Key) {
        if let PluginStatus::Dash { input } = &mut self.status {
            // todo: proper input handling - use some crate for that
            match key {
                Key::Esc => {
                    if let Some(pane) = &self.prev_focus {
                        pane.id().focus();
                    }
                    self.status = PluginStatus::Editor;
                }
                Key::Backspace | Key::Delete => {
                    input.pop();
                }
                Key::BackTab if input.is_empty() => {
                    if let Some(last) = &self.last_label_input {
                        *input = last.clone();
                    }
                }
                Key::Char(c) => {
                    input.push(c);
                    *input = input.trim().to_string();
                }
                _ => {}
            }
        }
    }
}
