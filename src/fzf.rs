use std::{num::ParseIntError, str::FromStr};

use itertools::Itertools;
use zellij_tile::prelude::CommandToRun;

use crate::{
    input::DASH_CMD,
    message::{MessageType, MSG_CLIENT_ID_ARG},
    PluginState, PLUGIN_NAME,
};

impl PluginState {
    pub(crate) fn get_fzf_pane_cmd<'a>(
        &self,
        options: impl Iterator<Item = &'a str>,
        message_type: MessageType,
    ) -> CommandToRun {
        let opts = options.into_iter().join("\n");
        let cmd = format!(
                    "printf '{opts}' | command cat -n | {DASH_CMD} | awk '{{print $1}}' | zellij pipe --plugin {PLUGIN_NAME} --name {} --args '{MSG_CLIENT_ID_ARG}={}'",
                    message_type.as_ref(),
                    self.msg_client_id
                );
        CommandToRun {
            // path: "fish".into(),
            path: "bash".into(),
            args: vec!["-c".to_string(), cmd],
            cwd: None,
        }
    }

    pub(crate) fn parse_fzf_index<T>(payload: &str) -> Option<T>
    where
        T: FromStr<Err = ParseIntError>,
    {
        payload.lines().next().and_then(|l| l.parse::<T>().ok())
    }
}
