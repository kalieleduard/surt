use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::fields;
use super::files::SchemaSource;

mod checks;
mod deliveries;
mod delivering;
mod names;
mod profile;

pub(crate) use deliveries::DeliveryCheck;
pub(crate) use profile::{Auth, ClaudeProfile, ConversationScope, Profile};
use profile::{Effort, PermissionMode, RuntimeKind};

const DEFAULT_OUTCOME: &str = "done";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentContract {
    schema: i64,
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcomes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) inputs: BTreeMap<String, InputSpec>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) outputs: BTreeMap<String, OutputSpec>,
    pub(crate) profile: Profile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Format {
    Text,
    Markdown,
    Json,
    ItemList,
    Questions,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InputSpec {
    format: Format,
    #[serde(default)]
    pub(crate) optional: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    json_schema: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputSpec {
    pub(crate) format: Format,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    outcomes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) json_schema: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) item_files: BTreeMap<String, ItemFileSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    checks: Vec<DeliveryCheck>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemFileSpec {
    pub(crate) format: Format,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) json_schema: Option<String>,
}

impl AgentContract {
    pub(crate) fn parse(
        id: &str,
        toml: &str,
        schemas: &impl SchemaSource,
    ) -> Result<AgentContract, Box<InvalidDefinition>> {
        AgentContract::checked(id, toml, schemas).map_err(|problem| {
            let field = fields::agent_field(&problem, toml);
            InvalidDefinition::new(DefinitionFile::Agent(id.to_owned()), None, problem)
                .with_field(field)
        })
    }

    fn checked(
        id: &str,
        toml: &str,
        schemas: &impl SchemaSource,
    ) -> Result<AgentContract, Problem> {
        let contract: AgentContract =
            toml::from_str(toml).map_err(|error| Problem::syntax(toml, &error))?;
        contract.check(id, schemas)?;
        Ok(contract)
    }

    pub(crate) fn frozen(text: &str) -> Result<AgentContract, toml::de::Error> {
        toml::from_str(text)
    }

    pub(crate) fn model(&self) -> &str {
        &self.profile.model
    }

    pub(crate) fn outcomes(&self) -> Vec<String> {
        self.outcomes
            .clone()
            .unwrap_or_else(|| vec![DEFAULT_OUTCOME.to_owned()])
    }

    pub(crate) fn canonical(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }

    pub(crate) fn schema_paths(&self) -> impl Iterator<Item = &str> {
        let inputs = self
            .inputs
            .values()
            .filter_map(|input| input.json_schema.as_deref());
        let outputs = self.outputs.values().flat_map(|output| {
            output.json_schema.as_deref().into_iter().chain(
                output
                    .item_files
                    .values()
                    .filter_map(|file| file.json_schema.as_deref()),
            )
        });
        inputs.chain(outputs)
    }
}
