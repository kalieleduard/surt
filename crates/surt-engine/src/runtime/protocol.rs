use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::AgentLaunch;

#[derive(Clone, Debug)]
pub struct RuntimeRequest {
    #[cfg_attr(
        not(feature = "test-support"),
        expect(
            dead_code,
            reason = "only the Fake Runtime looks an Execution up by step, Iteration and Attempt"
        )
    )]
    pub(crate) step: StepAttempt,
    pub(crate) session_id: Uuid,
    pub(crate) model: String,
    pub(crate) delivery_dir: PathBuf,
    pub(crate) launch: AgentLaunch,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepAttempt {
    pub(crate) step: String,
    pub(crate) iteration: u32,
    pub(crate) attempt: u32,
}

impl StepAttempt {
    pub fn new(step: &str, iteration: u32, attempt: u32) -> StepAttempt {
        StepAttempt {
            step: step.to_owned(),
            iteration,
            attempt,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeCompletion {
    pub(crate) exit: Exit,
    pub(crate) initialization: Option<Initialization>,
    pub(crate) result: Option<FinalResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Initialization {
    pub(crate) session_id: Uuid,
    pub(crate) model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinalResult {
    pub(crate) succeeded: bool,
    pub(crate) structured_output: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    Launched { pid: Option<u32> },
    Initialized(Initialization),
}

pub type RuntimeEventSink = UnboundedSender<RuntimeEvent>;
