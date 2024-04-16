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
                    && 
                        // not the current focused pane 
                        self.current_focus != PaneFocus::from(*p) 
            })
            .cloned()
    }

    pub(crate) fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        self.prev_focus = Some(std::mem::replace(
            &mut self.current_focus,
            focused_pane.into(),
        ));

        if !self.status.dashing()
            && !self.status.filepicking()
            && self.current_focus.floating()
            && self.current_focus.id() == self.dash_pane_id
        {
            self.status = PluginStatus::Dash {
                input: String::default(),
            };
        }
    }

    pub(crate) fn focus_editor_pane(&self) {
        // if self.current_focus.id() != self.editor_pane_id {
        self.editor_pane_id.focus();
        // }
    }
}
