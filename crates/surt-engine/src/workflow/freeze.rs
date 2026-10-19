use std::io;
use std::path::PathBuf;

use super::closure::{self, Closure};
use super::definition::{Definition, OnSuccess};
use super::error::{AGENT_FILE, INSTRUCTIONS_FILE, InvalidDefinition};
use super::files::DefinitionSource;
use super::references::local_references;
use super::steps::StepKind;
use crate::artifacts::{identity, sha256_hex};

const WORKFLOW_FILE: &str = "workflow.toml";
const CONFIG_FILE: &str = "config.toml";
const CONFIG_NAME: &str = "config";

#[derive(Debug, thiserror::Error)]
pub(crate) enum FreezeError {
    #[error("invalid definition: {0:?}")]
    Invalid(Box<InvalidDefinition>),
    #[error("cannot write the canonical form: {0}")]
    Canonical(#[from] toml::ser::Error),
    #[error("cannot read {}", path.display())]
    Unreadable {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl From<Box<InvalidDefinition>> for FreezeError {
    fn from(invalid: Box<InvalidDefinition>) -> FreezeError {
        FreezeError::Invalid(invalid)
    }
}

pub(crate) struct FrozenRun {
    pub(crate) definition_sha256: String,
    pub(crate) config_sha256: String,
    pub(crate) snapshots: Vec<Snapshot>,
    pub(crate) steps: Vec<TopLevelStep>,
}

pub(crate) struct TopLevelStep {
    pub(crate) key: String,
    pub(crate) kind: StepKind,
}

pub(crate) struct Snapshot {
    pub(crate) kind: SnapshotKind,
    pub(crate) name: String,
    pub(crate) sha256: String,
    pub(crate) files: Vec<SnapshotFile>,
}

pub(crate) struct SnapshotFile {
    pub(crate) rel_path: String,
    pub(crate) sha256: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotKind {
    Workflow,
    Agent,
    Config,
}

impl SnapshotKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SnapshotKind::Workflow => "workflow",
            SnapshotKind::Agent => "agent",
            SnapshotKind::Config => "config",
        }
    }
}

impl SnapshotFile {
    fn new(rel_path: &str, bytes: Vec<u8>) -> SnapshotFile {
        SnapshotFile {
            rel_path: rel_path.to_owned(),
            sha256: sha256_hex(&bytes),
            bytes,
        }
    }
}

pub(crate) fn freeze<'a>(
    source: &impl DefinitionSource,
    workflow: &str,
    initial_artifacts: impl IntoIterator<Item = &'a str>,
    config: impl FnOnce(OnSuccess) -> Result<String, toml::ser::Error>,
) -> Result<FrozenRun, FreezeError> {
    let closure = closure::resolve(source, workflow, initial_artifacts)?;
    let root = workflow_snapshot(&closure.root)?;
    let definition_sha256 = root.sha256.clone();
    let mut snapshots = vec![root];
    for child in &closure.children {
        snapshots.push(workflow_snapshot(child)?);
    }
    snapshots.extend(agent_snapshots(&closure)?);
    let config = config(closure.root.on_success())?;
    let config = single_file(SnapshotKind::Config, CONFIG_NAME, CONFIG_FILE, config);
    let config_sha256 = config.sha256.clone();
    snapshots.push(config);
    let steps = top_level_steps(&closure.root);
    Ok(FrozenRun {
        definition_sha256,
        config_sha256,
        snapshots,
        steps,
    })
}

fn workflow_snapshot(definition: &Definition) -> Result<Snapshot, FreezeError> {
    Ok(single_file(
        SnapshotKind::Workflow,
        &definition.name,
        WORKFLOW_FILE,
        definition.canonical()?,
    ))
}

fn single_file(kind: SnapshotKind, name: &str, rel_path: &str, text: String) -> Snapshot {
    let file = SnapshotFile::new(rel_path, text.into_bytes());
    Snapshot {
        kind,
        name: name.to_owned(),
        sha256: file.sha256.clone(),
        files: vec![file],
    }
}

fn agent_snapshots(closure: &Closure) -> Result<Vec<Snapshot>, FreezeError> {
    let mut snapshots = Vec::new();
    for (id, contract) in &closure.agents {
        let Some(sources) = closure.files.get(id) else {
            continue;
        };
        let mut files = vec![
            SnapshotFile::new(AGENT_FILE, contract.canonical()?.into_bytes()),
            SnapshotFile::new(INSTRUCTIONS_FILE, sources.instructions.as_bytes().to_vec()),
        ];
        let mut pending: Vec<String> = contract.schema_paths().map(str::to_owned).collect();
        while let Some(path) = pending.pop() {
            let Some(text) = sources.schema(&path) else {
                continue;
            };
            if files.iter().any(|file| file.rel_path == path) {
                continue;
            }
            pending.extend(local_references(&path, text));
            files.push(SnapshotFile::new(&path, text.as_bytes().to_vec()));
        }
        snapshots.push(Snapshot {
            kind: SnapshotKind::Agent,
            name: id.clone(),
            sha256: identity(
                files
                    .iter()
                    .map(|file| (file.rel_path.as_str(), file.sha256.as_str())),
            ),
            files,
        });
    }
    Ok(snapshots)
}

fn top_level_steps(definition: &Definition) -> Vec<TopLevelStep> {
    definition
        .steps
        .iter()
        .filter(|step| definition.enclosing_loop(&step.key).is_none())
        .map(|step| TopLevelStep {
            key: step.key.clone(),
            kind: step.spec.kind(),
        })
        .collect()
}
