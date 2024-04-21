use core::panic;
use kdl::KdlDocument;
use std::collections::HashMap;
use zellij_tile::{
    prelude::PaneInfo,
    shim::{dump_session_layout, hide_self},
};

use crate::{pane::PaneId, PluginState};

const LAYOUT_CWD_PLACEHOLDER: &str = "___layout_cwd___";

impl PluginState {
    // todo: this will go to project
    // pub(crate) fn check_itialised(&mut self, panes: &HashMap<usize, Vec<PaneInfo>>) -> bool {
    //     if self.initialised() {
    //         return true;
    //     }

    //     if matches!(
    //         self.init,
    //         Some(PluginInit {
    //             editor_pane_id: None,
    //             ..
    //         })
    //     ) {
    //         if let Some(pane) = panes.values().flatten().find(|p| p.title == "editor") {
    //             dump_session_layout();
    //             self.set_editor_pane_id(pane.into());
    //             return self.initialised();
    //         }
    //     }

    //     false
    // }

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
