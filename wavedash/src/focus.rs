use utils::pane::PaneFocus;
use zellij_tile::prelude::PaneInfo;

use crate::{input::KeybindPane,  PluginState}; 

impl PluginState {
    pub(crate) fn check_focus_change(&mut self) {
        if let Some(focused_pane) = self.has_focus_changed(&self.active_project().all_focused_panes) {
            self.on_focus_change(&focused_pane);
        }
    }

    pub(crate) fn has_focus_changed(&self, tab_panes: &[PaneInfo]) -> Option<PaneInfo> {
        let proj = self.active_project();
        tab_panes
            .iter()
            .find(|p| {
                p.is_focused
                    // both a tiled and a floating pane can be focused (but only the top one is relevant here)
                    && p.is_floating == proj.floating
                    && 
                        // not the current focused pane 
                        proj.current_focus != PaneFocus::from(*p)
            })
            .cloned()
    }

    pub(crate) fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        let focus: PaneFocus = focused_pane.into();
        self.handle_focus_change(focus.clone());
        let proj = self.active_project_mut();
        proj.current_focus = focus;
        if let Some(id) = proj.keybind_panes.get(&KeybindPane::StatusPaneDash) {
            if id != &proj.current_focus.id() {
                // reset dash pane to refresh fzf list
                // todo: might need a more general approach for all fzf & other refreshable panes
                id.close();
                proj.keybind_panes.remove(&KeybindPane::StatusPaneDash);
            }
        }
    }

    pub(crate) fn focus_editor_pane(&self) {
        self.active_project().editor_pane_id.focus();
    }
}
