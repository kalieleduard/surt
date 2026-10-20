mod conversation;
mod delivery;
mod scripted;
mod worktree;

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use super::{
    AgentRuntime, Exit, FailureClass, FinalResult, Initialization, RuntimeCompletion, RuntimeError,
    RuntimeEvent, RuntimeEventSink, RuntimeReport, RuntimeRequest, STREAM_LOG, StepAttempt,
    StopSignal,
};

use conversation::Turn;
pub use delivery::Delivery;
use delivery::FinalJson;
pub use scripted::InvalidScript;
use worktree::WorktreeCommit;

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
    LimitFailureIn {
        seconds: i64,
    },
    WaitUntilStopped,
    TranscriptMissing(Delivery),
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
        let turn = behavior.turn(&request);
        log(&request.log_dir, &turn.line())?;
        for event in [
            RuntimeEvent::Launched { pid: None },
            RuntimeEvent::Initialized(initialization.clone()),
        ] {
            log(
                &request.log_dir,
                &json!({ "type": "fake", "event": format!("{event:?}") }),
            )?;
            let _ = events.send(event);
        }
        let played = behavior.play(&request, initialization.clone(), &stop).await;
        let report = simulated(initialization, &played, turn);
        let _ = events.send(RuntimeEvent::Reported(report));
        played
    }
}

fn simulated(
    initialization: Initialization,
    played: &Result<RuntimeCompletion, RuntimeError>,
    turn: Turn,
) -> RuntimeReport {
    RuntimeReport {
        session_id: Some(initialization.session_id.to_string()),
        model: Some(initialization.model),
        exit_code: played
            .as_ref()
            .ok()
            .and_then(|completion| completion.exit.code()),
        conversation: turn.conversation,
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
            Behavior::Deliver(delivery)
            | Behavior::ModelMismatch { delivery, .. }
            | Behavior::TranscriptMissing(delivery) => {
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

fn log(log_dir: &Path, line: &Value) -> Result<(), RuntimeError> {
    fs::create_dir_all(log_dir).map_err(write_failure)?;
    let mut stream = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join(STREAM_LOG))
        .map_err(write_failure)?;
    writeln!(stream, "{line}").map_err(write_failure)
}

fn write_failure(error: std::io::Error) -> RuntimeError {
    RuntimeError::new(FailureClass::Retryable, "fake_write", error.to_string())
}
