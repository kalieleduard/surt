use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{Behavior, Delivery, FinalJson, Script, WorktreeCommit};

const FIRST: u32 = 1;

#[derive(Debug, thiserror::Error)]
#[error("invalid Fake Runtime script: {0}")]
pub struct InvalidScript(#[from] toml::de::Error);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptFile {
    #[serde(default)]
    on: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    step: String,
    #[serde(default = "first")]
    iteration: u32,
    #[serde(default = "first")]
    attempt: u32,
    #[serde(flatten)]
    behavior: ScriptedBehavior,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ScriptedBehavior {
    Deliver(ScriptedDelivery),
    ModelMismatch {
        reported: String,
        delivery: ScriptedDelivery,
    },
    Crash {
        exit_code: i32,
    },
    LimitFailure {
        #[serde(default)]
        resets_at: Option<i64>,
        #[serde(default)]
        resets_in: Option<i64>,
    },
    WaitUntilStopped {},
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptedDelivery {
    outcome: String,
    #[serde(default)]
    outputs: Map<String, Value>,
    #[serde(default)]
    files: BTreeMap<String, String>,
    final_json: Option<String>,
    commit: Option<ScriptedCommit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptedCommit {
    message: String,
    files: BTreeMap<String, String>,
}

fn first() -> u32 {
    FIRST
}

impl Script {
    pub fn from_toml(text: &str) -> Result<Script, InvalidScript> {
        let file: ScriptFile = toml::from_str(text)?;
        Ok(file.on.into_iter().fold(Script::new(), |script, entry| {
            script.on(
                &entry.step,
                entry.iteration,
                entry.attempt,
                entry.behavior.into(),
            )
        }))
    }
}

impl From<ScriptedBehavior> for Behavior {
    fn from(scripted: ScriptedBehavior) -> Behavior {
        match scripted {
            ScriptedBehavior::Deliver(delivery) => Behavior::Deliver(delivery.into()),
            ScriptedBehavior::ModelMismatch { reported, delivery } => Behavior::ModelMismatch {
                reported,
                delivery: delivery.into(),
            },
            ScriptedBehavior::Crash { exit_code } => Behavior::Crash { exit_code },
            ScriptedBehavior::LimitFailure {
                resets_in: Some(seconds),
                ..
            } => Behavior::LimitFailureIn { seconds },
            ScriptedBehavior::LimitFailure { resets_at, .. } => {
                Behavior::LimitFailure { resets_at }
            }
            ScriptedBehavior::WaitUntilStopped {} => Behavior::WaitUntilStopped,
        }
    }
}

impl From<ScriptedDelivery> for Delivery {
    fn from(scripted: ScriptedDelivery) -> Delivery {
        Delivery {
            outcome: scripted.outcome,
            outputs: scripted.outputs,
            files: bytes(scripted.files),
            final_json: match scripted.final_json {
                Some(text) => FinalJson::Text(Some(text)),
                None => FinalJson::Envelope,
            },
            commit: scripted.commit.map(|commit| WorktreeCommit {
                message: commit.message,
                files: bytes(commit.files),
            }),
        }
    }
}

fn bytes(files: BTreeMap<String, String>) -> Vec<(String, Vec<u8>)> {
    files
        .into_iter()
        .map(|(path, contents)| (path, contents.into_bytes()))
        .collect()
}
