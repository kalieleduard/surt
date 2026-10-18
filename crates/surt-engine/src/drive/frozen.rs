use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::path::PathBuf;

use serde::Deserialize;

use super::decide::Limits;
use crate::artifacts::DeliveryContract;
use crate::checks::CheckSpec;
use crate::project::{Check, Error, parse_timeout};
use crate::runtime::AgentLaunch;
use crate::store::FrozenFile;
use crate::workflow::{AgentContract, Definition, StepSpec};

const WORKFLOW_FILE: &str = "workflow.toml";
const AGENT_FILE: &str = "agent.toml";
const CONFIG_FILE: &str = "config.toml";
const INSTRUCTIONS_FILE: &str = "instructions.md";

pub(crate) struct Frozen {
    definitions: BTreeMap<String, Definition>,
    agents: BTreeMap<String, FrozenAgent>,
    pub(crate) limits: Limits,
    pub(crate) checks: Vec<CheckSpec>,
}

pub(crate) struct FrozenAgent {
    pub(crate) snapshot: String,
    pub(crate) contract: AgentContract,
    files: BTreeMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
#[error("the frozen snapshot {name} is unreadable: {detail}")]
pub(crate) struct CorruptSnapshot {
    name: String,
    detail: String,
}

#[derive(Deserialize)]
struct FrozenConfig {
    max_attempts: NonZeroU32,
    max_iterations: NonZeroU32,
    #[serde(default)]
    checks: Vec<Check>,
}

struct Files {
    kind: String,
    name: String,
    files: BTreeMap<String, String>,
}

impl CorruptSnapshot {
    pub(crate) fn new(name: &str, detail: impl ToString) -> CorruptSnapshot {
        CorruptSnapshot {
            name: name.to_owned(),
            detail: detail.to_string(),
        }
    }
}

impl From<CorruptSnapshot> for Error {
    fn from(corrupt: CorruptSnapshot) -> Error {
        Error::internal(corrupt)
    }
}

impl Frozen {
    pub(crate) fn from_files(rows: Vec<FrozenFile>) -> Result<Frozen, CorruptSnapshot> {
        let mut definitions = BTreeMap::new();
        let mut agents = BTreeMap::new();
        let mut config = None;
        for (snapshot, files) in group(rows)? {
            match files.kind.as_str() {
                "workflow" => {
                    let definition = Definition::parse(&files.name, files.text(WORKFLOW_FILE)?)
                        .map_err(|invalid| {
                            CorruptSnapshot::new(&files.name, format!("{invalid:?}"))
                        })?;
                    definitions.insert(snapshot, definition);
                }
                "agent" => {
                    let contract = AgentContract::frozen(files.text(AGENT_FILE)?)
                        .map_err(|error| CorruptSnapshot::new(&files.name, error))?;
                    let agent = FrozenAgent {
                        snapshot,
                        contract,
                        files: files.files,
                    };
                    agents.insert(files.name, agent);
                }
                _ => config = Some(frozen_config(&files)?),
            }
        }
        let (limits, checks) =
            config.ok_or_else(|| CorruptSnapshot::new(CONFIG_FILE, "missing"))?;
        Ok(Frozen {
            definitions,
            agents,
            limits,
            checks,
        })
    }

    pub(crate) fn definition(&self, snapshot: &str) -> Result<&Definition, CorruptSnapshot> {
        self.definitions
            .get(snapshot)
            .ok_or_else(|| CorruptSnapshot::new(snapshot, "no such workflow"))
    }

    pub(crate) fn definition_named(
        &self,
        name: &str,
    ) -> Result<(&str, &Definition), CorruptSnapshot> {
        self.definitions
            .iter()
            .find(|(_, definition)| definition.name == name)
            .map(|(snapshot, definition)| (snapshot.as_str(), definition))
            .ok_or_else(|| CorruptSnapshot::new(name, "no such workflow"))
    }

    pub(crate) fn agent(&self, name: &str) -> Result<&FrozenAgent, CorruptSnapshot> {
        self.agents
            .get(name)
            .ok_or_else(|| CorruptSnapshot::new(name, "no such agent"))
    }
}

impl FrozenAgent {
    pub(crate) fn launch(
        &self,
        working_dir: PathBuf,
        launch_dir: PathBuf,
    ) -> Result<AgentLaunch, CorruptSnapshot> {
        let instructions = self
            .files
            .get(INSTRUCTIONS_FILE)
            .ok_or_else(|| CorruptSnapshot::new(&self.snapshot, "no instructions"))?;
        Ok(AgentLaunch::new(
            &self.contract,
            instructions,
            working_dir,
            launch_dir,
        ))
    }

    pub(crate) fn delivery_contract(&self) -> Result<DeliveryContract, CorruptSnapshot> {
        DeliveryContract::new(&self.contract, |path| {
            self.files.get(path).map(String::as_str)
        })
        .map_err(|invalid| CorruptSnapshot::new(&self.snapshot, invalid))
    }
}

impl Files {
    fn text(&self, rel_path: &str) -> Result<&str, CorruptSnapshot> {
        self.files
            .get(rel_path)
            .map(String::as_str)
            .ok_or_else(|| CorruptSnapshot::new(&self.name, format!("{rel_path} is missing")))
    }
}

pub(crate) fn gate_artifacts(definition: &Definition) -> BTreeSet<&str> {
    definition
        .steps
        .iter()
        .filter_map(|step| match &step.spec {
            StepSpec::Approval(gate) => Some(gate.artifact.as_str()),
            _ => None,
        })
        .collect()
}

fn group(rows: Vec<FrozenFile>) -> Result<BTreeMap<String, Files>, CorruptSnapshot> {
    let mut grouped: BTreeMap<String, Files> = BTreeMap::new();
    for row in rows {
        let text = String::from_utf8(row.bytes).map_err(|_| {
            CorruptSnapshot::new(&row.name, format!("{} is not UTF-8", row.rel_path))
        })?;
        grouped
            .entry(row.snapshot)
            .or_insert_with(|| Files {
                kind: row.kind,
                name: row.name,
                files: BTreeMap::new(),
            })
            .files
            .insert(row.rel_path, text);
    }
    Ok(grouped)
}

fn frozen_config(files: &Files) -> Result<(Limits, Vec<CheckSpec>), CorruptSnapshot> {
    let config: FrozenConfig = toml::from_str(files.text(CONFIG_FILE)?)
        .map_err(|error| CorruptSnapshot::new(&files.name, error))?;
    let limits = Limits {
        max_attempts: count(config.max_attempts),
        max_iterations: count(config.max_iterations),
    };
    let checks = config
        .checks
        .into_iter()
        .map(|check| frozen_check(&files.name, check))
        .collect::<Result<_, _>>()?;
    Ok((limits, checks))
}

fn frozen_check(snapshot: &str, check: Check) -> Result<CheckSpec, CorruptSnapshot> {
    let timeout = check
        .timeout
        .as_deref()
        .map(|text| {
            parse_timeout(text)
                .ok_or_else(|| CorruptSnapshot::new(snapshot, format!("timeout {text:?}")))
        })
        .transpose()?;
    Ok(CheckSpec {
        name: check.name,
        run: check.run,
        timeout,
    })
}

fn count(limit: NonZeroU32) -> usize {
    usize::try_from(limit.get()).unwrap_or(usize::MAX)
}
