use zellij_tile::prelude::Key;

use crate::{PluginState, PluginStatus};

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

impl PluginState {
    pub(crate) fn handle_key(&mut self, key: Key) {
        match &self.status {
            PluginStatus::FilePicker => self.handle_filepicker_key(key),
            _ => {}
        }
    }

    fn handle_filepicker_key(&mut self, key: Key) {
        match key {
            Key::Esc => {
                if let Some(id) = self
                    .keybind_panes
                    .get(&crate::message::KeybindPane::FilePicker)
                {
                    id.close();
                    self.status = PluginStatus::Editor;
                }
            }
            _ => {}
        }
    }
}
