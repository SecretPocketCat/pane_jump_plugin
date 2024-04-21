use core::panic;
use kdl::KdlDocument;
use std::collections::HashMap;
use zellij_tile::{
    prelude::PaneInfo,
    shim::{dump_session_layout, hide_self},
};

use crate::{pane::PaneId, PluginState};

const LAYOUT_CWD_PLACEHOLDER: &str = "___layout_cwd___";

#[derive(Debug, Default, PartialEq)]
pub(crate) struct PluginInit {
    tab: Option<usize>,
    editor_pane_id: Option<PaneId>,
    new_tab_layout: Option<String>,
}

macro_rules! init_plugin_field {
    ($param: ident, $t: ty, $fn: ident) => {
        impl PluginState {
            pub(crate) fn $fn(&mut self, $param: $t) {
                if let Some(init) = &mut self.init {
                    init.$param = Some($param);

                    if let (Some(tab), Some(editor_pane_id), Some(new_tab_layout)) =
                        (init.tab, init.editor_pane_id, &init.new_tab_layout)
                    {
                        self.tab = tab;
                        self.editor_pane_id = editor_pane_id;
                        self.new_tab_layout = new_tab_layout.to_string();
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
init_plugin_field!(new_tab_layout, String, set_new_tab_layout);

impl PluginState {
    pub(crate) fn initialised(&self) -> bool {
        !self.init.is_some()
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
                eprintln!("Tab: {tab}");

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
                dump_session_layout();
                self.set_editor_pane_id(pane.into());
                return self.initialised();
            }
        }

        false
    }

    pub(crate) fn format_layout(layout: String) -> String {
        match layout.parse::<KdlDocument>() {
            Ok(layout_doc) => {
                // let layout_node = layout_doc.get("layout").unwrap();
                // let tab_node = layout_node.get("tab").unwrap();
                // // todo:
                // let floating_panes = tab_node.("tab").unwrap();

                // // todo: clear wavedash floating pane
                // // set cwd
                // // dot't start panes suspedned?
                // let floating = tab_node.get("floating_panes").unwrap();

                layout_doc.to_string()
            }
            Err(e) => {
                eprintln!("Failed to parse layout: {e}");
                panic!("Invalid layout");
            }
        }
    }

    pub(crate) fn layout(&self, cwd: &str) -> String {
        let layout = self.new_tab_layout.to_string();
        let layout = layout.replace(LAYOUT_CWD_PLACEHOLDER, cwd);
        // todo: clear wavedash floating pane
        // set cwd
        // dot't start panes suspedned?

        layout
    }
}
