use command_queue::CommandQueue;
use init::PluginInit;
use message::KeybindPane;
use pane::{PaneFocus, PaneId};
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;
use wavedash::DashPane;
use zellij_tile::prelude::*;

mod command_queue;
mod focus;
mod init;
mod input;
mod message;
mod pane;
mod render;
mod utils;
mod wavedash;

const PLUGIN_NAME: &str = "wavedash";

#[derive(Debug, PartialEq)]
enum PluginStatus {
    Init(PluginInit),
    Editor,
    FilePicker,
    Dash { input: String },
}

struct PluginState {
    status: PluginStatus,
    tab: usize,
    editor_pane_id: PaneId,
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: PaneFocus,
    prev_focus: Option<PaneFocus>,
    all_focused_panes: Vec<PaneInfo>,
    dash_panes: Vec<DashPane>,
    last_label_input: Option<String>,
    dash_pane_id: PaneId,
    palette: Palette,
    columns: usize,
    rows: usize,
    msg_client_id: Uuid,
    command_queue: CommandQueue,
    keybind_panes: HashMap<KeybindPane, PaneId>,
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: PluginStatus::Init(Default::default()),
            tab: 0,
            editor_pane_id: PaneId::Terminal(0),
            floating: true,
            current_focus: PaneFocus::Tiled(PaneId::Terminal(0)),
            prev_focus: None,
            all_focused_panes: Default::default(),
            dash_panes: Default::default(),
            last_label_input: None,
            dash_pane_id: PaneId::Plugin(0),
            palette: Default::default(),
            columns: 0,
            rows: 0,
            msg_client_id: Uuid::new_v4(),
            command_queue: Default::default(),
            keybind_panes: Default::default(),
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
            PermissionType::OpenTerminalsOrPlugins,
            PermissionType::RunCommands,
            PermissionType::WriteToStdin,
        ]);
        subscribe(&[
            EventType::Key,
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::ModeUpdate,
            EventType::Timer,
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
            Event::Timer(_) => self.process_timer(),
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
