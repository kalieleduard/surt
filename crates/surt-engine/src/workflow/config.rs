use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RunConfig {
    pub(crate) max_attempts: u32,
    pub(crate) max_iterations: u32,
    pub(crate) git: GitConfig,
    pub(crate) checks: Vec<Check>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct GitConfig {
    pub(crate) merge_into_base: MergeIntoBase,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Project::open builds it from config.toml")
)]
pub(crate) enum MergeIntoBase {
    Never,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Check {
    pub(crate) name: String,
    pub(crate) run: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeout: Option<String>,
}

impl RunConfig {
    pub(crate) fn canonical(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}
