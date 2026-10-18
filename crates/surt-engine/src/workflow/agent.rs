use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::catalog::AgentAssets;
use super::error::{DefinitionFile, InvalidDefinition, Problem};

mod checks;
mod names;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    runtime: RuntimeKind,
    pub(crate) model: String,
    pub(crate) effort: Effort,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) limits: Option<Limits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) claude: Option<ClaudeProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
enum RuntimeKind {
    #[serde(rename = "claude-code")]
    ClaudeCode,
    #[serde(rename = "fake")]
    Fake,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_turns: Option<NonZeroU32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) max_budget_usd: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaudeProfile {
    pub(crate) auth: Auth,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) tools: Vec<String>,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) disallowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) api_key_env: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Auth {
    Subscription,
    ApiKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum PermissionMode {
    #[serde(rename = "dontAsk")]
    DontAsk,
    #[serde(rename = "bypassPermissions")]
    BypassPermissions,
}

impl AgentContract {
    pub(crate) fn parse(
        id: &str,
        toml: &str,
        assets: &AgentAssets,
    ) -> Result<AgentContract, Box<InvalidDefinition>> {
        AgentContract::parse_effective(id, toml, assets).map_err(|problem| {
            InvalidDefinition::new(DefinitionFile::Agent(id.to_owned()), None, problem)
        })
    }

    pub(crate) fn parse_effective(
        id: &str,
        toml: &str,
        assets: &AgentAssets,
    ) -> Result<AgentContract, Problem> {
        let contract: AgentContract =
            toml::from_str(toml).map_err(|error| Problem::syntax(toml, &error))?;
        contract.check(id, assets)?;
        Ok(contract)
    }

    pub(crate) fn frozen(text: &str) -> Result<AgentContract, toml::de::Error> {
        toml::from_str(text)
    }

    pub(crate) fn model(&self) -> &str {
        &self.profile.model
    }

    pub(crate) fn runtime(&self) -> &'static str {
        match self.profile.runtime {
            RuntimeKind::ClaudeCode => "claude-code",
            RuntimeKind::Fake => "fake",
        }
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
