use std::{num::ParseIntError, str::FromStr};

use itertools::Itertools;
use uuid::Uuid;
use zellij_tile::{prelude::CommandToRun, shim::run_command};

pub const MSG_CLIENT_ID_ARG: &str = "msg_client_id";

pub fn get_fzf_pane_cmd<'a>(
    options: impl Iterator<Item = &'a str>,
    plugin_name: impl Into<&'a str>,
    message_type: impl Into<&'a str>,
    message_client_id: Uuid,
) -> CommandToRun {
    let opts = options.into_iter().join("\n");
    let cmd = format!(
        "printf '{opts}' | command cat -n | fzf --layout reverse --with-nth 2.. | awk '{{print $1}}' | zellij pipe --plugin {} --name {} --args '{MSG_CLIENT_ID_ARG}={message_client_id}'",
        plugin_name.into(),
        message_type.into());
    CommandToRun {
        // path: "fish".into(),
        path: "bash".into(),
        args: vec!["-c".to_string(), cmd],
        cwd: None,
    }
}

pub fn parse_fzf_index<T>(payload: &str) -> Option<T>
where
    T: FromStr<Err = ParseIntError>,
{
    payload.lines().next().and_then(|l| l.parse::<T>().ok())
}

pub fn run_find_repos_command<'a>(cwd: impl Into<&'a str>) {
    run_command(
        &[
            "find",
            cwd.into(),
            "-type",
            "d",
            "-exec",
            "test",
            "-d",
            "{}/.git",
            ";",
            "-prune",
            "-print",
        ],
        Default::default(),
    );
}
