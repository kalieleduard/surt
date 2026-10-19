use std::collections::BTreeMap;

use super::catalog::AgentAssets;

#[derive(Clone, Debug, Default)]
pub(crate) struct LocalAssets {
    pub(crate) instructions: Option<String>,
    pub(crate) schemas: BTreeMap<String, String>,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentSources<'a> {
    pub(crate) assets: &'a AgentAssets,
    pub(crate) local: Option<&'a LocalAssets>,
}

pub(crate) trait SchemaSource {
    fn schema_text(&self, path: &str) -> Option<&str>;
}

impl SchemaSource for AgentAssets {
    fn schema_text(&self, path: &str) -> Option<&str> {
        self.schema(path)
    }
}

impl SchemaSource for AgentSources<'_> {
    fn schema_text(&self, path: &str) -> Option<&str> {
        self.schema(path)
    }
}

impl<'a> AgentSources<'a> {
    pub(crate) fn instructions(&self) -> &'a str {
        self.local
            .and_then(|local| local.instructions.as_deref())
            .unwrap_or(self.assets.instructions)
    }

    pub(crate) fn schema(&self, path: &str) -> Option<&'a str> {
        self.local
            .and_then(|local| local.schemas.get(path))
            .map(String::as_str)
            .or_else(|| self.assets.schema(path))
    }
}
