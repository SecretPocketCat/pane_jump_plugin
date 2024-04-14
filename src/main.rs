use file_picker::PickerStatus;
use init::PluginInit;
use pane::{PaneFocus, PaneId};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;
use wavedash::DashPane;
use zellij_tile::prelude::*;

mod file_picker;
mod focus;
mod init;
mod input;
mod message;
mod pane;
mod render;
mod utils;
mod wavedash;

// todo: move some PluginState fields into the variants
#[derive(Debug, PartialEq)]
enum PluginStatus {
    Init(PluginInit),
    Editor,
    FilePicker(PickerStatus),
    Dash { input: String },
}

impl PluginStatus {
    pub(crate) fn dashing(&self) -> bool {
        matches!(self, Self::Dash { .. })
    }
}

struct PluginState {
    status: PluginStatus,
    tab: usize,
    editor_pane_id: PaneId,
    git_pane_id: Option<PaneId>,
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
    msg_client_id: Uuid,
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: PluginStatus::Init(Default::default()),
            tab: 0,
            editor_pane_id: PaneId::Terminal(0),
            git_pane_id: None,
            floating: true,
            current_focus: PaneFocus::Tiled(PaneId::Terminal(0)),
            prev_focus: None,
            last_focused_editor: None,
            all_focused_panes: Default::default(),
            dash_panes: Default::default(),
            last_label_input: None,
            dash_pane_id: PaneId::Plugin(0),
            palette: Default::default(),
            columns: 0,
            rows: 0,
            msg_client_id: Uuid::new_v4(),
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
            Event::Key(key) => self.handle_key(key),
            Event::ModeUpdate(ModeInfo { style, .. }) => {
                if !self.initialised() {
                    self.set_palette(style.colors);
                }
            }
            Event::TabUpdate(tabs) => self.handle_tab_update(&tabs),
            Event::PaneUpdate(panes) => self.handle_pane_update(panes),
            _ => unimplemented!("{event:?}"),
        }

        self.should_render()
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.render_pane(rows, cols);
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        self.handle_pipe_message(pipe_message)
    }
}
