use std::collections::HashMap;
use zellij_tile::{prelude::PaneInfo, shim::hide_self};

use crate::{pane::PaneId, PluginState};

#[derive(Debug, Default, PartialEq)]
pub(crate) struct PluginInit {
    tab: Option<usize>,
    editor_pane_id: Option<PaneId>,
}

macro_rules! init_plugin_field {
    ($param: ident, $t: ty, $fn: ident) => {
        impl PluginState {
            pub(crate) fn $fn(&mut self, $param: $t) {
                if let Some(init) = &mut self.init {
                    init.$param = Some($param);

                    if let (Some(tab), Some(editor_pane_id)) = (init.tab, init.editor_pane_id) {
                        self.tab = tab;
                        self.editor_pane_id = editor_pane_id;
                        self.init.take();
                        hide_self();
                    }
                } else {
                    self.$param = $param;
                    return;
                }
            }
        }
    };
}

init_plugin_field!(tab, usize, set_tab);
init_plugin_field!(editor_pane_id, PaneId, set_editor_pane_id);

impl PluginState {
    pub(crate) fn initialised(&self) -> bool {
        if let Some(init) = &self.init {
            init.tab.is_some() && init.editor_pane_id.is_some()
        } else {
            true
        }
    }

    pub(crate) fn check_itialised(&mut self, panes: &HashMap<usize, Vec<PaneInfo>>) -> bool {
        if self.initialised() {
            return true;
        }

        if matches!(self.init, Some(PluginInit { tab: None, .. })) {
            if let Some(tab) = panes
                .iter()
                .find(|(_, panes)| panes.iter().any(|p| PaneId::from(p) == self.dash_pane_id))
                .map(|(tab, _)| *tab)
            {
                self.set_tab(tab);
                return self.initialised();
            }
        }

        if matches!(
            self.init,
            Some(PluginInit {
                editor_pane_id: None,
                ..
            })
        ) {
            if let Some(pane) = panes.values().flatten().find(|p| p.title == "editor") {
                self.set_editor_pane_id(pane.into());
                return self.initialised();
            }
        }

        false
    }
}
