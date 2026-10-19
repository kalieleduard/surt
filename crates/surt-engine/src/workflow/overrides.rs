mod blame;
mod fields;
mod schemas;

use std::collections::BTreeMap;

use toml::Table;

use super::agent::AgentContract;
use super::catalog::Catalog;
use super::error::Problem;
use super::sources::{AgentSources, LocalAssets};

const TEST_ONLY_RUNTIME: &str = "fake";

pub(crate) struct LocalAgent {
    pub(crate) id: String,
    pub(crate) toml: Option<LocalFile>,
    pub(crate) instructions: Option<LocalFile>,
    pub(crate) schemas: BTreeMap<String, LocalFile>,
}

pub(crate) struct LocalFile {
    pub(crate) file: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOverride {
    pub file: String,
    pub field: Option<String>,
    pub needed_by: Option<WorkflowStep>,
    pub problem: Problem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowStep {
    pub workflow: String,
    pub step: String,
    pub field: Option<StepUse>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepUse {
    When,
    Until,
    Input,
    DeliveryCheck,
}

impl InvalidOverride {
    pub(crate) fn new(file: &str, field: Option<String>, problem: Problem) -> Box<Self> {
        Box::new(InvalidOverride {
            file: file.to_owned(),
            field,
            needed_by: None,
            problem,
        })
    }
}

impl Catalog {
    pub(crate) fn with_local(
        &self,
        locals: &[LocalAgent],
    ) -> Result<Catalog, Box<InvalidOverride>> {
        let mut effective = self.clone();
        for local in locals {
            let Some(assets) = self.agent_assets(&local.id) else {
                continue;
            };
            let local_assets = local.assets();
            let sources = AgentSources {
                assets,
                local: Some(&local_assets),
            };
            schemas::check(local, &sources)?;
            if let Some(merged) = effective_toml(local, &sources)? {
                effective = effective.with_effective_toml(&local.id, merged);
            }
            effective = effective.with_local_assets(&local.id, local_assets);
        }
        Ok(effective)
    }
}

fn effective_toml(
    local: &LocalAgent,
    sources: &AgentSources,
) -> Result<Option<String>, Box<InvalidOverride>> {
    let Some(file) = &local.toml else {
        return Ok(None);
    };
    let Ok(builtin) = toml::from_str::<Table>(sources.assets.toml) else {
        return Ok(None);
    };
    let merged = merged_toml(builtin, file)?;
    AgentContract::parse_effective(&local.id, &merged, sources)
        .and_then(production_runtime)
        .map_err(|problem| {
            InvalidOverride::new(
                &file.file,
                fields::field_of(&problem, &merged),
                fields::without_line(problem),
            )
        })?;
    Ok(Some(merged))
}

impl LocalAgent {
    fn assets(&self) -> LocalAssets {
        LocalAssets {
            instructions: self.instructions.as_ref().map(|file| file.text.clone()),
            schemas: self
                .schemas
                .iter()
                .map(|(path, file)| (path.clone(), file.text.clone()))
                .collect(),
        }
    }
}

fn production_runtime(contract: AgentContract) -> Result<AgentContract, Problem> {
    if cfg!(feature = "test-support") || contract.runtime() != TEST_ONLY_RUNTIME {
        return Ok(contract);
    }
    Err(Problem::TestOnlyRuntime)
}

fn merged_toml(mut merged: Table, file: &LocalFile) -> Result<String, Box<InvalidOverride>> {
    let invalid = |problem| InvalidOverride::new(&file.file, None, problem);
    let local: Table =
        toml::from_str(&file.text).map_err(|error| invalid(Problem::syntax(&file.text, &error)))?;
    merge(&mut merged, local);
    toml::to_string(&merged).map_err(|error| {
        invalid(Problem::Syntax {
            line: None,
            detail: error.to_string(),
        })
    })
}

fn merge(base: &mut Table, local: Table) {
    for (key, value) in local {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(inner)), toml::Value::Table(local_inner)) => {
                merge(inner, local_inner);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}
