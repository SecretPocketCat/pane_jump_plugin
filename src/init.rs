use std::collections::HashMap;
use zellij_tile::prelude::{Palette, PaneInfo};

use crate::{pane::PaneId, PluginState, PluginStatus};

#[derive(Debug, Default, PartialEq)]
pub(crate) struct PluginInit {
    columns: Option<usize>,
    rows: Option<usize>,
    palette: Option<Palette>,
    tab: Option<usize>,
    editor_pane_id: Option<PaneId>,
}

macro_rules! init_plugin_field {
    ($param: ident, $t: ty, $fn: ident) => {
        impl PluginState {
            pub(crate) fn $fn(&mut self, $param: $t) {
                if let PluginStatus::Init(init) = &mut self.status {
                    init.$param = Some($param);

                    if let (
                        Some(columns),
                        Some(rows),
                        Some(palette),
                        Some(tab),
                        Some(editor_pane_id),
                    ) = (
                        init.columns,
                        init.rows,
                        init.palette,
                        init.tab,
                        init.editor_pane_id,
                    ) {
                        self.columns = columns;
                        self.rows = rows;
                        self.palette = palette;
                        self.tab = tab;
                        self.editor_pane_id = editor_pane_id;
                        self.status = PluginStatus::Editor;
                    }
                } else {
                    self.$param = $param;
                    return;
                }
            }
        }
    };
}

init_plugin_field!(rows, usize, set_rows);
init_plugin_field!(columns, usize, set_columns);
init_plugin_field!(palette, Palette, set_palette);
init_plugin_field!(tab, usize, set_tab);
init_plugin_field!(editor_pane_id, PaneId, set_editor_pane_id);

impl PluginState {
    pub(crate) fn initialised(&self) -> bool {
        if let PluginStatus::Init(init) = &self.status {
            init.columns.is_some()
                && init.rows.is_some()
                && init.palette.is_some()
                && init.tab.is_some()
                && init.editor_pane_id.is_some()
        } else {
            true
        }
    }

    pub(crate) fn check_itialised(&mut self, panes: &HashMap<usize, Vec<PaneInfo>>) -> bool {
        if self.initialised() {
            return true;
        }

        if matches!(
            self.status,
            PluginStatus::Init(PluginInit { tab: None, .. })
        ) {
            if let Some(tab) = panes
                .iter()
                .find(|(_, panes)| panes.iter().any(|p| &PaneId::from(p) == &self.dash_pane_id))
                .map(|(tab, _)| *tab)
            {
                self.set_tab(tab);
                return self.initialised();
            }
        }

        if matches!(
            self.status,
            PluginStatus::Init(PluginInit {
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
