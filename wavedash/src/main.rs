use command_queue::CommandQueue;
use indexmap::IndexMap;
use input::KeybindPane;
use std::collections::{BTreeMap, HashMap};
use tracing::{info, instrument, warn};
use tracing_subscriber::{fmt, prelude::*};
use utils::{
    pane::{PaneFocus, PaneId},
    project::{ProjectOption, ProjectRootConfiguration, PROJECT_ROOT_RESP_MESSAGE_NAME},
};
use uuid::Uuid;
use zellij_tile::prelude::*;

mod command_queue;
mod focus;
mod input;
mod message;
mod pane;
mod project;

#[derive(Debug)]
pub(crate) struct ProjectTab {
    title: String,
    idx: usize,
    editor_pane_id: Option<PaneId>,
    // not part of focus fields because it's part of `TabUpdate`
    floating: bool,
    current_focus: Option<PaneFocus>,
    status_panes: IndexMap<PaneId, String>,
    terminal_panes: IndexMap<PaneId, String>,
    keybind_panes: HashMap<KeybindPane, PaneId>,
    spawned_extra_term_count: usize,
}

impl ProjectTab {
    pub(crate) fn uninit(&self) -> bool {
        self.editor_pane_id.is_none()
    }
}

struct PluginState {
    tab: Option<String>,
    projects: HashMap<String, ProjectTab>,
    project_options: Vec<ProjectOption>,
    plugin_id: PaneId,
    msg_client_id: Uuid,
    command_queue: CommandQueue,
    queued_pane_update: Option<PaneManifest>,
    queued_tab_update: Option<Vec<TabInfo>>,
    root_config: Option<ProjectRootConfiguration>,
}

impl PluginState {
    pub(crate) fn project_uninit(&self) -> bool {
        !self
            .tab
            .as_ref()
            .is_some_and(|t| self.projects.contains_key(t))
    }
}

// there's a bunch of sentinel values, but those are part of the init state to make workind with those more ergonomic as those fields should be always set after init
impl Default for PluginState {
    fn default() -> Self {
        Self {
            tab: None,
            projects: Default::default(),
            project_options: Default::default(),
            plugin_id: PaneId::Plugin(0),
            msg_client_id: Uuid::new_v4(),
            command_queue: Default::default(),
            queued_pane_update: Default::default(),
            queued_tab_update: Default::default(),
            root_config: None,
        }
    }
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    #[instrument(skip(self))]
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let appender = tracing_appender::rolling::never("/host/target", "log");
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(appender))
            .init();

        self.plugin_id = PaneId::new(get_plugin_ids().plugin_id, true);
        show_self(true);
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::OpenTerminalsOrPlugins,
            PermissionType::RunCommands,
            PermissionType::WriteToStdin,
            PermissionType::MessageAndLaunchOtherPlugins,
        ]);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::Timer,
            EventType::RunCommandResult,
        ]);
        info!(plugin_id=?self.plugin_id, msg_client_id=?self.msg_client_id, "Wavedash plugin load");
    }

    #[instrument(skip(self))]
    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                self.queued_tab_update = Some(tabs);
                self.command_queue
                    .queue_timer_command(command_queue::QueuedTimerCommand::ProcessQueuedTabUpdate);
            }
            Event::Timer(_) => self.handle_timer(),
            Event::RunCommandResult(exit_code, stdout, stderr, _ctx) => {
                self.handle_command_result(exit_code, stdout, stderr)
            }
            Event::PaneUpdate(pane_update) => self.queued_pane_update = Some(pane_update.clone()),
            _ => unimplemented!("{event:?}"),
        }

        false
    }

    #[instrument(skip_all)]
    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if self.project_uninit() && pipe_message.name != PROJECT_ROOT_RESP_MESSAGE_NAME {
            warn!(tab = self.tab, "Tab not initialized yet");
            return false;
        }

        self.handle_pipe_message(pipe_message)
    }
}
