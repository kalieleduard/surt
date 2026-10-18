mod scripted;
mod worktree;

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::{
    AgentRuntime, Exit, FailureClass, FinalResult, Initialization, RuntimeCompletion, RuntimeError,
    RuntimeEvent, RuntimeEventSink, RuntimeReport, RuntimeRequest, STREAM_LOG, StepAttempt,
    StopSignal,
};

pub use scripted::InvalidScript;
use worktree::{WorktreeCommit, resolve_head};

const SUCCESS_EXIT_CODE: i32 = 0;

#[derive(Clone, Debug)]
pub struct FakeRuntime {
    script: Script,
}

#[derive(Clone, Debug, Default)]
pub struct Script {
    behaviors: BTreeMap<StepAttempt, Behavior>,
}

#[derive(Clone, Debug)]
pub enum Behavior {
    Deliver(Delivery),
    ModelMismatch {
        reported: String,
        delivery: Delivery,
    },
    Crash {
        exit_code: i32,
    },
    LimitFailure {
        resets_at: Option<i64>,
    },
    /// A limit failure whose reset time is `seconds` after it happens.
    LimitFailureIn {
        seconds: i64,
    },
    WaitUntilStopped,
}

#[derive(Clone, Debug)]
pub struct Delivery {
    outcome: String,
    outputs: Map<String, Value>,
    files: Vec<(String, Vec<u8>)>,
    final_json: FinalJson,
    commit: Option<WorktreeCommit>,
}

#[derive(Clone, Debug)]
enum FinalJson {
    Envelope,
    Text(Option<String>),
}

impl FakeRuntime {
    pub fn new(script: Script) -> FakeRuntime {
        FakeRuntime { script }
    }
}

impl Script {
    pub fn new() -> Script {
        Script::default()
    }

    pub fn on(mut self, step: &str, iteration: u32, attempt: u32, behavior: Behavior) -> Script {
        self.behaviors
            .insert(StepAttempt::new(step, iteration, attempt), behavior);
        self
    }
}

impl Delivery {
    pub fn new(outcome: &str) -> Delivery {
        Delivery {
            outcome: outcome.to_owned(),
            outputs: Map::new(),
            files: Vec::new(),
            final_json: FinalJson::Envelope,
            commit: None,
        }
    }

    pub fn output(self, name: &str, path: &str, contents: impl Into<Vec<u8>>) -> Delivery {
        self.output_without_file(name, path).file(path, contents)
    }

    pub fn output_without_file(mut self, name: &str, path: &str) -> Delivery {
        self.outputs
            .insert(name.to_owned(), Value::String(path.to_owned()));
        self
    }

    pub fn file(mut self, path: &str, contents: impl Into<Vec<u8>>) -> Delivery {
        self.files.push((path.to_owned(), contents.into()));
        self
    }

    pub fn final_json(mut self, text: Option<&str>) -> Delivery {
        self.final_json = FinalJson::Text(text.map(str::to_owned));
        self
    }

    fn final_text(&self) -> Option<String> {
        match &self.final_json {
            FinalJson::Envelope => Some(self.envelope()),
            FinalJson::Text(text) => text.clone(),
        }
    }

    fn envelope(&self) -> String {
        let mut envelope = Map::new();
        envelope.insert("outcome".to_owned(), Value::String(self.outcome.clone()));
        envelope.insert("outputs".to_owned(), Value::Object(self.outputs.clone()));
        Value::Object(envelope).to_string()
    }

    async fn write(&self, delivery_dir: &Path, worktree: &Path) -> Result<(), RuntimeError> {
        if let Some(commit) = &self.commit {
            commit.make(worktree).await?;
        }
        for (path, contents) in &self.files {
            let target = delivery_dir.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(write_failure)?;
            }
            fs::write(&target, resolve_head(contents, worktree).await?).map_err(write_failure)?;
        }
        Ok(())
    }
}

impl AgentRuntime for FakeRuntime {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let behavior = self.script.behaviors.get(&request.step).ok_or_else(|| {
            RuntimeError::new(
                FailureClass::NotRetryable,
                "unscripted",
                format!("no scripted behaviour for {:?}", request.step),
            )
        })?;
        let initialization = Initialization {
            session_id: request.session_id,
            model: behavior.reported_model(&request.model),
        };
        for event in [
            RuntimeEvent::Launched { pid: None },
            RuntimeEvent::Initialized(initialization.clone()),
        ] {
            log(&request.log_dir, &event)?;
            let _ = events.send(event);
        }
        let played = behavior.play(&request, initialization.clone(), &stop).await;
        let _ = events.send(RuntimeEvent::Reported(simulated(initialization, &played)));
        played
    }
}

fn simulated(
    initialization: Initialization,
    played: &Result<RuntimeCompletion, RuntimeError>,
) -> RuntimeReport {
    RuntimeReport {
        session_id: Some(initialization.session_id.to_string()),
        model: Some(initialization.model),
        exit_code: played
            .as_ref()
            .ok()
            .and_then(|completion| completion.exit.code()),
        ..RuntimeReport::default()
    }
}

impl Behavior {
    fn reported_model(&self, requested: &str) -> String {
        match self {
            Behavior::ModelMismatch { reported, .. } => reported.clone(),
            _ => requested.to_owned(),
        }
    }

    async fn play(
        &self,
        request: &RuntimeRequest,
        initialization: Initialization,
        stop: &StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let ended = |exit, result| RuntimeCompletion {
            exit,
            initialization: Some(initialization),
            result,
        };
        match self {
            Behavior::Deliver(delivery) | Behavior::ModelMismatch { delivery, .. } => {
                delivery
                    .write(&request.delivery_dir, &request.launch.working_dir)
                    .await?;
                let result = FinalResult {
                    succeeded: true,
                    structured_output: delivery.final_text(),
                };
                Ok(ended(Exit::Code(SUCCESS_EXIT_CODE), Some(result)))
            }
            Behavior::Crash { exit_code } => Ok(ended(Exit::Code(*exit_code), None)),
            Behavior::LimitFailure { resets_at } => Err(RuntimeError::new(
                FailureClass::Transient,
                "rate_limit",
                "the scripted usage limit was reached",
            )
            .resetting_at(*resets_at)),
            Behavior::LimitFailureIn { seconds } => Err(RuntimeError::new(
                FailureClass::Transient,
                "rate_limit",
                "the scripted usage limit was reached",
            )
            .resetting_at(Some(jiff::Timestamp::now().as_second() + seconds))),
            Behavior::WaitUntilStopped => {
                stop.requested().await;
                Ok(ended(Exit::Stopped, None))
            }
        }
    }
}

fn log(log_dir: &Path, event: &RuntimeEvent) -> Result<(), RuntimeError> {
    fs::create_dir_all(log_dir).map_err(write_failure)?;
    let mut stream = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join(STREAM_LOG))
        .map_err(write_failure)?;
    let line = json!({ "type": "fake", "event": format!("{event:?}") });
    writeln!(stream, "{line}").map_err(write_failure)
}

fn write_failure(error: std::io::Error) -> RuntimeError {
    RuntimeError::new(FailureClass::Retryable, "fake_write", error.to_string())
}
