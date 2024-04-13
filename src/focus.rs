use zellij_tile::prelude::PaneInfo;

use crate::{pane::PaneFocus, PluginState, PluginStatus};

impl PluginState {
    pub(crate) fn check_focus_change(&mut self) {
        if let Some(focused_pane) = self.has_focus_changed(&self.all_focused_panes) {
            self.on_focus_change(&focused_pane);
        }
    }

    pub(crate) fn has_focus_changed(&self, tab_panes: &[PaneInfo]) -> Option<PaneInfo> {
        tab_panes
            .iter()
            .find(|p| {
                p.is_focused
                    // both a tiled and a floating pane can be focused (but only the top one is relevant here)
                    && p.is_floating == self.floating
                    && (
                        // not the current focused pane or `last_focused_editor` has not been set yet
                        self.current_focus != PaneFocus::from(*p) || self.last_focused_editor.is_none())
            })
            .cloned()
    }

    pub(crate) fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        self.prev_focus = Some(std::mem::replace(
            &mut self.current_focus,
            focused_pane.into(),
        ));

        if let Some(last_focused_editor) = &self.last_focused_editor {
            if let Some(current_dash_pane) = self.dash_panes.get(&self.current_focus.id()) {
                if current_dash_pane.editor && last_focused_editor != &self.current_focus {
                    last_focused_editor.id().hide();
                }
            }
        }

        if let Some(current_pane) = self.dash_panes.get(&self.current_focus.id()) {
            if current_pane.editor {
                self.last_focused_editor = Some(self.current_focus.clone());
            }
        }

        if !self.status.dashing()
            && self.current_focus.floating()
            && self.current_focus.id() == self.dash_pane_id
        {
            self.status = PluginStatus::Dash {
                input: String::default(),
            };
        }
    }
}
