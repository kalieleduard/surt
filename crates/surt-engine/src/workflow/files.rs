use std::collections::BTreeMap;

use super::freeze::FreezeError;

#[derive(Clone, Debug, Default)]
pub(crate) struct AgentFiles {
    pub(crate) toml: String,
    pub(crate) instructions: String,
    pub(crate) schemas: BTreeMap<String, String>,
}

pub(crate) trait DefinitionSource {
    fn workflow(&self, name: &str) -> Result<Option<String>, FreezeError>;

    fn agent(&self, id: &str) -> Result<Option<AgentFiles>, FreezeError>;
}

pub(crate) trait SchemaSource {
    fn schema_text(&self, path: &str) -> Option<&str>;
}

impl AgentFiles {
    pub(crate) fn schema(&self, path: &str) -> Option<&str> {
        self.schemas.get(path).map(String::as_str)
    }

    #[cfg(all(test, feature = "test-support"))]
    pub(crate) fn of(toml: &str, instructions: &str, schemas: &[(&str, &str)]) -> AgentFiles {
        AgentFiles {
            toml: toml.to_owned(),
            instructions: instructions.to_owned(),
            schemas: schemas
                .iter()
                .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
                .collect(),
        }
    }
}

impl SchemaSource for AgentFiles {
    fn schema_text(&self, path: &str) -> Option<&str> {
        self.schema(path)
    }
}
