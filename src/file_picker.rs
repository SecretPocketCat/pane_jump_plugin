use zellij_tile::prelude::{CommandToRun, FloatingPaneCoordinates};
use zellij_tile::shim::open_command_pane_floating;

use crate::message::MSG_CLIENT_ID_ARG;
use crate::pane::PaneId;
use crate::{PluginState, PluginStatus};

// todo: rewrite this to actually write to the editor pane
// https://zellij.dev/documentation/plugin-api-commands#write_chars
// ideally send ESC too, to cancel out of any open tabs
// then send :open /file/path.smt - this focuses already opened buffers too
// so if yazi could support fzf within the current dir,
// then this could actually be used as a launcher
// however, switching tabs like this actually loses tab info,
// so would have to either use bufferjump list when it exists or parse
// cursor position from the status line and then restore that or
// patch hx to restore the position when this occurs

// this also means hiding editor panes is no longer needed as only 1 pane is required

#[derive(Default, Debug, PartialEq, Clone)]
pub(crate) enum PickerStatus {
    #[default]
    Idle,
    OpeningPicker,
    Picking(PaneId),
}

impl PluginState {
    pub(crate) fn open_picker(&mut self) {
        self.status = PluginStatus::FilePicker(PickerStatus::OpeningPicker);
        open_command_pane_floating(
                CommandToRun {
                    path: "bash".into(),
                    args: vec![
                        "-c".to_string(),
                        format!("yazi --chooser-file /dev/stdout | zellij pipe --plugin file_picker --name file --args '{MSG_CLIENT_ID_ARG}={}'", self.msg_client_id),
                    ],
                    cwd: None,
                },
                Some(
                    FloatingPaneCoordinates::default()
                        .with_x_fixed(0)
                        .with_y_fixed(0)
                        .with_width_percent(95)
                        .with_height_percent(90),
                ),
            );
    }
}
