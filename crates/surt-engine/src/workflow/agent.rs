use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::catalog::AgentAssets;
use super::error::{DefinitionFile, InvalidDefinition, Problem};

mod checks;

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
    profile: Profile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Format {
    Text,
    Markdown,
    Json,
    ItemList,
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
    pub(crate) json_schema: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) item_files: BTreeMap<String, ItemFileSpec>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemFileSpec {
    pub(crate) format: Format,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) json_schema: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    runtime: RuntimeKind,
    model: String,
    effort: Effort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    limits: Option<Limits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    claude: Option<ClaudeProfile>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
enum RuntimeKind {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "fake")]
    Fake,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_turns: Option<NonZeroU32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_budget_usd: Option<f64>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ClaudeProfile {
    auth: Auth,
    permission_mode: PermissionMode,
    tools: Vec<String>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    api_key_env: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Auth {
    Subscription,
    ApiKey,
}

#[derive(Deserialize, Serialize)]
enum PermissionMode {
    #[serde(rename = "dontAsk")]
    DontAsk,
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

impl AgentContract {
    pub(crate) fn parse(
        id: &str,
        assets: &AgentAssets,
    ) -> Result<AgentContract, Box<InvalidDefinition>> {
        let invalid =
            |problem| InvalidDefinition::new(DefinitionFile::Agent(id.to_owned()), None, problem);
        let contract: AgentContract = toml::from_str(assets.toml)
            .map_err(|error| invalid(Problem::syntax(assets.toml, &error)))?;
        contract.check(id, assets).map_err(invalid)?;
        Ok(contract)
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
