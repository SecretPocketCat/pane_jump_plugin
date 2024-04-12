use init::PluginInit;
use pane::{PaneFocus, PaneId};
use std::collections::{BTreeMap, HashMap};
use wavedash::DashPane;
use zellij_tile::prelude::*;

mod file_picker;
mod focus;
mod init;
mod input;
mod pane;
mod render;
mod utils;
mod wavedash;

// todo: move some PluginState fields into the variants
#[derive(Debug, PartialEq)]
enum PluginStatus {
    Init(PluginInit),
    FilePicker,
    Editor,
    Dash { input: String },
}

struct PluginState {
    status: PluginStatus,
    tab: usize,
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: PaneFocus,
    prev_focus: Option<PaneFocus>,
    last_focused_editor: Option<PaneFocus>,
    all_focused_panes: Vec<PaneInfo>,
    dash_panes: HashMap<PaneId, DashPane>,
    last_label_input: Option<String>,
    dash_pane_id: PaneId,
    palette: Palette,
    columns: usize,
    rows: usize,

    // todo: replace by fuzzy search
    dash_pane_labels: HashMap<String, PaneId>,
    label_len: u8,
    label_alphabet: Vec<char>,
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: PluginStatus::Init(Default::default()),
            tab: 0,
            floating: true,
            current_focus: PaneFocus::Tiled(PaneId::Terminal(0)),
            prev_focus: None,
            last_focused_editor: None,
            all_focused_panes: Default::default(),
            dash_panes: Default::default(),
            dash_pane_labels: Default::default(),
            label_len: 1,
            last_label_input: None,
            label_alphabet: "tnseriaoplfuwyzkbvm".chars().collect(),
            dash_pane_id: PaneId::Plugin(0),
            palette: Default::default(),
            columns: 0,
            rows: 0,
        }
    }
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.dash_pane_id = PaneId::new(get_plugin_ids().plugin_id, true);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        subscribe(&[
            EventType::Key,
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ModeUpdate,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => return self.handle_key(key),
            Event::ModeUpdate(ModeInfo { style, .. }) => {
                if !self.initialised() {
                    self.set_palette(style.colors);
                }

                return true;
            }
            Event::TabUpdate(tabs) => return self.handle_tab_update(&tabs),
            Event::PaneUpdate(panes) => return self.handle_pane_update(panes),
            _ => unimplemented!("{event:?}"),
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.render_pane(rows, cols);
    }
}
