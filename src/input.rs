use zellij_tile::prelude::Key;

use crate::{file_picker::PickerStatus, pane::PaneFocus, PluginState, PluginStatus};

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
                    if self.label_len == 1 {
                        *input = c.to_string();
                    } else {
                        input.push(c);
                        *input = input.trim().to_string();
                    }

                    if let Some(pane) = self.dash_pane_labels.get(input) {
                        pane.focus();
                        self.current_focus = PaneFocus::new(pane.clone(), false);
                        self.status = PluginStatus::Editor;
                        // todo:
                        // self.last_label_input = Some(input.clone());
                    }
                }
                _ => {}
            }
        }
    }
}
