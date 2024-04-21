use command_queue::CommandQueue;
use indexmap::IndexMap;
use input::KeybindPane;
use std::collections::{BTreeMap, HashMap};
use utils::pane::{PaneFocus, PaneId};
use uuid::Uuid;
use zellij_tile::prelude::*;

mod command_queue;
mod focus;
mod init;
mod input;
mod message;
mod pane;
mod project;

const PLUGIN_NAME: &str = "wavedash";

struct ProjectTab {
    title: String,
    editor_pane_id: PaneId,
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: PaneFocus,
    all_focused_panes: Vec<PaneInfo>,
    status_panes: IndexMap<PaneId, String>,
    terminal_panes: IndexMap<PaneId, String>,
    keybind_panes: HashMap<KeybindPane, PaneId>,
    spawned_extra_term_count: usize,
}

struct PluginState {
    tab: usize,
    projects: IndexMap<usize, ProjectTab>,
    plugin_id: PaneId,
    msg_client_id: Uuid,
    command_queue: CommandQueue,
    new_tab_layout: String,
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            tab: 0,
            projects: Default::default(),
            plugin_id: PaneId::Plugin(0),
            msg_client_id: Uuid::new_v4(),
            command_queue: Default::default(),
            new_tab_layout: Default::default(),
        }
    }
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.plugin_id = PaneId::new(get_plugin_ids().plugin_id, true);
        show_self(true);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::OpenTerminalsOrPlugins,
            PermissionType::RunCommands,
            PermissionType::WriteToStdin,
        ]);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::CustomMessage,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => self.handle_tab_update(&tabs),
            Event::PaneUpdate(panes) => self.handle_pane_update(panes),
            Event::Timer(_) => self.handle_timer(),
            Event::RunCommandResult(exit_code, stdout, stderr, _ctx) => {
                self.handle_command_result(exit_code, stdout, stderr)
            }
            Event::CustomMessage(message, payload) => {
                // if message == "session_layout" {
                //     self.set_new_tab_layout(Self::format_layout(payload));
                // }
            }
            _ => unimplemented!("{event:?}"),
        }

        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        self.handle_pipe_message(pipe_message)
    }
}
