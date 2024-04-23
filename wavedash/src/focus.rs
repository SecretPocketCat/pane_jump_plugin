use utils::pane::PaneFocus;
use zellij_tile::prelude::PaneInfo;

use crate::{input::KeybindPane,  PluginState}; 

impl PluginState {
    pub(crate) fn check_focus_change(&mut self, tab_panes: &[PaneInfo]) {
        if let Some(focused_pane) = self.has_focus_changed(tab_panes) {
            self.on_focus_change(&focused_pane);
        }
    }

    fn has_focus_changed(&self, tab_panes: &[PaneInfo]) -> Option<PaneInfo> {
        if self.project_uninit() {
            return None;    
        }
        
        let proj = self.active_project().unwrap();
        tab_panes
            .iter()
            .find(|p| {
                p.is_focused
                    // both a tiled and a floating pane can be focused (but only the top one is relevant here)
                    && p.is_floating == proj.floating
                    && 
                        // not the current focused pane 
                        (proj.current_focus.is_none() ||
                        proj.current_focus.as_ref().is_some_and(|f| f != &PaneFocus::from(*p)))
            })
            .cloned()
    }

    fn on_focus_change(&mut self, focused_pane: &PaneInfo) {
        let focus: PaneFocus = focused_pane.into();
        self.handle_focus_change(focus.clone());
        let proj = self.active_project_mut().unwrap();
        proj.current_focus = Some(focus.clone());
        if let Some(id) = proj.keybind_panes.get(&KeybindPane::StatusPaneDash) {
            if id != &focus.id() {
                // reset dash pane to refresh fzf list
                // todo: might need a more general approach for all fzf & other refreshable panes
                id.close();
                proj.keybind_panes.remove(&KeybindPane::StatusPaneDash);
            }
        }
    }

    pub(crate) fn focus_editor_pane(&self) {
        if let Some(id) = self.active_project().and_then(|p| p.editor_pane_id) {
            eprintln!("Focusing pane - pane: {id:?}, tab {:?}, {}", self.tab, self.active_project().unwrap().title);
            id.focus();
        }
    }
}
