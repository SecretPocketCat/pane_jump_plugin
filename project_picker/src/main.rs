use configuration::ProjectPickerConfiguration;
use std::collections::BTreeMap;
use utils::{
    fzf::{get_fzf_pane_cmd, run_find_repos_command},
    message::MSG_CLIENT_ID_ARG,
    pane::PaneId,
    project::{
        parse_configuration, project_title, ProjectRootConfiguration,
        PROJECT_ROOT_RESP_MESSAGE_NAME, PROJECT_ROOT_RQST_MESSAGE_NAME,
    },
    template::wavedash_template,
    PROJECT_PICKER_PLUGIN_NAME,
};
use uuid::Uuid;
use zellij_tile::prelude::*;

mod configuration;

#[derive(Default)]
enum PluginStatus {
    #[default]
    Init,
    Picking(bool),
    Picked(bool),
    InvalidConfig(String),
}

struct PluginState {
    status: PluginStatus,
    pane_id: PaneId,
    msg_client_id: Uuid,
    cwd: String,
    projects_paths: Vec<String>,
    project_root: Option<ProjectRootConfiguration>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            status: Default::default(),
            pane_id: PaneId::Terminal(0),
            msg_client_id: Uuid::new_v4(),
            cwd: Default::default(),
            projects_paths: Default::default(),
            project_root: None,
        }
    }
}

impl PluginState {
    fn show_project_selection(&self) {
        open_command_pane_in_place(get_fzf_pane_cmd(
            self.projects_paths
                .iter()
                .map(|p| project_title(p, self.project_root.as_ref().unwrap().root_path.clone())),
            "pick_project",
            self.msg_client_id,
            true,
        ));
    }

    fn pick_project(&mut self, cwd: &str) {
        let name = project_title(cwd, self.project_root.as_ref().unwrap().root_path.clone());
        let template = wavedash_template(&cwd, &name, true);
        new_tabs_with_layout(&template);
        self.status = PluginStatus::Picked(false);
    }
}

register_plugin!(PluginState);
impl ZellijPlugin for PluginState {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        show_self(true);
        match parse_configuration(&configuration) {
            Ok(roots) => match ProjectPickerConfiguration::new(roots) {
                Ok(conf) => {
                    let plug_ids = get_plugin_ids();
                    self.cwd = plug_ids.initial_cwd.to_string_lossy().into_owned();
                    self.pane_id = PaneId::Plugin(plug_ids.plugin_id);

                    self.project_root = Some(
                        conf.root(&get_plugin_ids().initial_cwd.to_string_lossy())
                            .clone(),
                    );
                    request_permission(&[
                        PermissionType::ReadApplicationState,
                        PermissionType::ChangeApplicationState,
                        PermissionType::RunCommands,
                        PermissionType::MessageAndLaunchOtherPlugins,
                    ]);
                    subscribe(&[EventType::PaneUpdate, EventType::RunCommandResult]);
                }
                Err(e) => self.status = PluginStatus::InvalidConfig(e.to_string()),
            },
            Err(e) => self.status = PluginStatus::InvalidConfig(e.to_string()),
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PaneUpdate(PaneManifest { panes }) => match self.status {
                PluginStatus::Init => {
                    let root = self
                        .project_root
                        .as_ref()
                        .unwrap()
                        .root_path
                        .to_string_lossy();
                    run_find_repos_command(&*root);
                    self.status = PluginStatus::Picking(false);
                }
                PluginStatus::Picking(false) => {
                    if let Some(pane) = panes.values().flatten().find(|p| {
                        p.terminal_command.is_some() && p.title != PROJECT_PICKER_PLUGIN_NAME
                    }) {
                        let id = PaneId::from(pane);
                        id.rename(PROJECT_PICKER_PLUGIN_NAME);
                        self.status = PluginStatus::Picking(true);
                    }
                }
                _ => {}
            },
            Event::RunCommandResult(exit_code, stdout, stderr, _ctx) => {
                if let PluginStatus::Picking(_) = self.status {
                    if exit_code.is_some_and(|c| c != 0) {
                        eprintln!(
                            "Command has failed - exit code: '{}', err: {}",
                            exit_code.unwrap(),
                            String::from_utf8_lossy(&stderr)
                        );
                    } else {
                        let mut projects: Vec<String> = String::from_utf8_lossy(&stdout)
                            .lines()
                            .map(Into::into)
                            .collect();
                        let extra_paths = self
                            .project_root
                            .as_ref()
                            .unwrap()
                            .extra_project_paths
                            .clone()
                            .into_iter()
                            .map(|p| p.to_string_lossy().to_string());
                        projects.sort_unstable();
                        projects.extend(extra_paths);

                        self.projects_paths = projects;
                        if self.projects_paths.len() == 1 {
                            let cwd = self.projects_paths[0].clone();
                            self.pick_project(&cwd);
                        } else {
                            let plug_cwd = &self.cwd;
                            if let Some(cwd) = self
                                .projects_paths
                                .iter()
                                .find(move |p| p == &plug_cwd)
                                .cloned()
                            {
                                self.pick_project(&cwd);
                            } else {
                                self.show_project_selection();
                            }
                        }
                    }
                }
            }
            _ => unimplemented!("{event:?}"),
        }

        false
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        match self.status {
            PluginStatus::Picking(_) => {
                if pipe_message
                    .args
                    .get(MSG_CLIENT_ID_ARG)
                    .is_some_and(|guid| guid == &self.msg_client_id.to_string())
                {
                    let cwd = pipe_message
                        .payload
                        .unwrap_or_default()
                        .lines()
                        .next()
                        .and_then(|l| {
                            l.parse::<usize>()
                                .and_then(|i| Ok(self.projects_paths.get(i - 1)))
                                .ok()
                                .flatten()
                        });
                    if let Some(cwd) = cwd {
                        self.pick_project(&cwd.clone());
                    } else {
                        // replace cancelled fzf pane with a new one
                        close_focus();
                        self.status = PluginStatus::Picking(false);
                        self.show_project_selection();
                    }
                }
            }
            PluginStatus::Picked(false) => {
                if let (PROJECT_ROOT_RQST_MESSAGE_NAME, PipeSource::Plugin(target_plugin_id)) =
                    (pipe_message.name.as_str(), pipe_message.source)
                {
                    self.status = PluginStatus::Picked(true);
                    let msg = MessageToPlugin::new(PROJECT_ROOT_RESP_MESSAGE_NAME)
                        .with_destination_plugin_id(target_plugin_id)
                        .with_payload(
                            serde_json::to_string(&self.project_root.clone().unwrap())
                                .expect("Failed to serialize project root"),
                        );
                    pipe_message_to_plugin(msg);
                }
            }
            _ => {}
        }

        false
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        if let PluginStatus::InvalidConfig(error) = &self.status {
            println!("{error}");
        }
    }
}
