use anyhow::bail;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

#[derive(strum_macros::EnumString, Debug, PartialEq)]
enum ConfigField {
    #[strum(serialize = "root")]
    Root,
    #[strum(serialize = "extra")]
    ExtraProject,
    #[strum(serialize = "task_proj")]
    TaskProject,
}

#[derive(Debug)]
pub struct ProjectConfiguration {
    pub root: PathBuf,
    pub extra_project_paths: Vec<PathBuf>,
    pub root_task_project_filter: String,
    pub nested_task_project_filters: HashMap<String, String>,
}

#[derive(Default)]
struct ParsedProjectConfiguration {
    root: Option<PathBuf>,
    extra_project_paths: Vec<PathBuf>,
    root_task_project_filter: Option<String>,
    nested_task_project_filters: HashMap<String, String>,
}

pub fn parse_configuration(
    plugin_configuration: &BTreeMap<String, String>,
) -> anyhow::Result<Vec<ProjectConfiguration>> {
    let mut partial_configs: HashMap<&str, ParsedProjectConfiguration> = HashMap::new();

    for (k, value) in plugin_configuration.iter() {
        if let Some((field, key)) = k.split_once('.') {
            if let Ok(field) = field.parse::<ConfigField>() {
                match field {
                    ConfigField::Root => {
                        partial_configs
                            .entry(key)
                            .and_modify(|conf| conf.root = Some(value.into()))
                            .or_insert_with(|| {
                                let mut conf = ParsedProjectConfiguration::default();
                                conf.root = Some(value.into());
                                conf
                            });
                    }
                    ConfigField::ExtraProject => {
                        if let Some((root, _)) = key.split_once('.') {
                            partial_configs
                                .entry(root)
                                .and_modify(|conf| conf.extra_project_paths.push(value.into()))
                                .or_insert_with(|| {
                                    let mut conf = ParsedProjectConfiguration::default();
                                    conf.extra_project_paths.push(value.into());
                                    conf
                                });
                        } else {
                            bail!("Invalid extra project key '{k}'");
                        }
                    }
                    ConfigField::TaskProject => {
                        if let Some((root, key)) = key.split_once('.') {
                            // nested task project
                            partial_configs
                                .entry(root)
                                .and_modify(|conf| {
                                    conf.nested_task_project_filters
                                        .insert(key.to_string(), value.to_string());
                                })
                                .or_insert_with(|| {
                                    let mut conf = ParsedProjectConfiguration::default();
                                    conf.nested_task_project_filters
                                        .insert(key.to_string(), value.to_string());
                                    conf
                                });
                        } else {
                            // root task project
                            partial_configs
                                .entry(key)
                                .and_modify(|conf| {
                                    conf.root_task_project_filter = Some(value.into())
                                })
                                .or_insert_with(|| {
                                    let mut conf = ParsedProjectConfiguration::default();
                                    conf.root_task_project_filter = Some(value.into());
                                    conf
                                });
                        }
                    }
                }
            } else {
                eprintln!("Unknown config field '{field:?}");
            }
        }
    }

    let configs: Result<Vec<_>, _> = partial_configs
        .into_iter()
        .map(|(root, c)| match (c.root, c.root_task_project_filter) {
            (None, None) => bail!("Missing root path & root task project filter for root '{root}'"),
            (None, Some(_)) => bail!("Missing root path for root '{root}'"),
            (Some(_), None) => bail!("Missing root path project filter for root '{root}'"),
            (Some(root), Some(root_task_project_filter)) => Ok(ProjectConfiguration {
                root,
                root_task_project_filter,
                extra_project_paths: c.extra_project_paths,
                nested_task_project_filters: c.nested_task_project_filters,
            }),
        })
        .collect();
    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn test_conf(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn default_test_conf(extra_pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut pairs = vec![("root.test", "~/test/path"), ("task_proj.test", "test")];
        pairs.extend(extra_pairs);
        test_conf(&pairs)
    }

    #[test_case(test_conf(&[]) => matches Ok(conf) if conf.is_empty())]
    #[test_case(default_test_conf(&[]) => matches Ok(conf) if conf.len() == 1)]
    #[test_case(default_test_conf(&[
            ("task_proj.test.1", "test1"),
            ("task_proj.test.2", "test2"),
    ]) => matches Ok(conf) if conf.len() == 1 && conf[0].nested_task_project_filters.len() == 2)]
    #[test_case(default_test_conf(&[
            ("extra.test.test1", "path/1"),
            ("extra.test.test2", "path/2"),
    ]) => matches Ok(conf) if conf.len() == 1 && conf[0].extra_project_paths.len() == 2)]
    #[test_case(test_conf(&[
        ("root.test", "~/test/path")
    ]) => matches Err(_))]
    #[test_case(test_conf(&[
        ("task_proj.test", "~/test/path")
    ]) => matches Err(_))]
    #[test_case(test_conf(&[
        ("extra.test", "/sub/path")
    ]) => matches Err(_))]
    #[test_case(test_conf(&[
        ("extra.test_2", "/sub/path")
    ]) => matches Err(_))]
    fn parse(
        plugin_configuration: BTreeMap<String, String>,
    ) -> anyhow::Result<Vec<ProjectConfiguration>> {
        parse_configuration(&plugin_configuration)
    }
}
