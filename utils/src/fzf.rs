use itertools::Itertools;
use std::{num::ParseIntError, str::FromStr};
use uuid::Uuid;
use zellij_tile::{prelude::CommandToRun, shim::run_command};

use crate::message::MSG_CLIENT_ID_ARG;

pub fn get_fzf_pane_cmd<'a>(
    options: impl Iterator<Item = &'a str>,
    message_type: impl Into<&'a str>,
    message_client_id: Uuid,
    use_index: bool,
) -> CommandToRun {
    let opts = options.into_iter().join("\n");

    let fzf_layout_args = "--layout reverse";
    let fzf_cmd = if use_index {
        format!("command cat -n | fzf {fzf_layout_args} --with-nth 2.. | awk '{{print $1}}'")
    } else {
        format!("fzf {fzf_layout_args} ")
    };

    let cmd = format!(
        "printf '{opts}' | {fzf_cmd} | zellij pipe  --name {} --args '{MSG_CLIENT_ID_ARG}={message_client_id}'",
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

// todo: look for project specific dirs or files like cargo.toml etc too
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
