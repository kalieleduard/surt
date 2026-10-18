mod fields;

use toml::Table;

use super::agent::AgentContract;
use super::catalog::Catalog;
use super::error::Problem;

const TEST_ONLY_RUNTIME: &str = "fake";

pub(crate) struct LocalAgent {
    pub(crate) id: String,
    pub(crate) toml: Option<LocalFile>,
}

pub(crate) struct LocalFile {
    pub(crate) file: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOverride {
    pub file: String,
    pub field: Option<String>,
    pub problem: Problem,
}

impl InvalidOverride {
    pub(crate) fn new(file: &str, field: Option<String>, problem: Problem) -> Box<Self> {
        Box::new(InvalidOverride {
            file: file.to_owned(),
            field,
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
            let Some(file) = &local.toml else {
                continue;
            };
            let Some(assets) = self.agent_assets(&local.id) else {
                continue;
            };
            let Ok(builtin) = toml::from_str::<Table>(assets.toml) else {
                continue;
            };
            let merged = merged_toml(builtin, file)?;
            AgentContract::parse_effective(&local.id, &merged, assets)
                .and_then(production_runtime)
                .map_err(|problem| {
                    InvalidOverride::new(
                        &file.file,
                        fields::field_of(&problem, &merged),
                        fields::without_line(problem),
                    )
                })?;
            effective = effective.with_effective_toml(&local.id, merged);
        }
        Ok(effective)
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
