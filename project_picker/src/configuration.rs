use utils::project::ProjectRootConfiguration;

#[derive(Debug)]
pub struct ProjectPickerConfiguration {
    pub roots: Vec<ProjectRootConfiguration>,
    default_idx: usize,
}

impl ProjectPickerConfiguration {
    pub fn new(roots: Vec<ProjectRootConfiguration>) -> anyhow::Result<Self> {
        let default: Vec<_> = roots
            .iter()
            .enumerate()
            .filter(|(_, r)| r.default)
            .map(|(i, _)| i)
            .collect();

        match default.len() {
            1 => Ok(ProjectPickerConfiguration {
                roots,
                default_idx: default[0],
            }),
            count => anyhow::bail!("There must be exactly 1 default root, but there're {count}"),
        }
    }

    pub fn default_root(&self) -> &ProjectRootConfiguration {
        &self.roots[self.default_idx]
    }

    pub fn root(&self, cwd: &str) -> &ProjectRootConfiguration {
        &self
            .roots
            .iter()
            .find(|r| r.root_path.to_string_lossy().contains(cwd))
            .unwrap_or_else(|| self.default_root())
    }
}
