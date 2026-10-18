use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::project::Refusal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveOutcome {
    AtGate { gates: Vec<Gate> },
    Succeeded,
    Blocked { steps: Vec<StepLabel> },
    Cancelled,
    Interrupted,
    Halted(Refusal),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gate {
    pub step: StepLabel,
    pub artifact: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepLabel {
    pub key: String,
    pub item: Option<String>,
    pub iteration: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    StepStarted {
        step: StepLabel,
        attempt: u32,
    },
    StepFinished {
        step: StepLabel,
        outcome: String,
    },
    AttemptFailed {
        step: StepLabel,
        attempt: u32,
        code: String,
        message: String,
    },
    AttemptInterrupted {
        step: StepLabel,
        attempt: u32,
    },
    RevisionPublished {
        artifact: String,
        revision: u32,
    },
    Salvaged {
        step: StepLabel,
        reference: String,
    },
    WorktreeRepaired {
        step: StepLabel,
        path: PathBuf,
    },
}

pub type ProgressSink = UnboundedSender<Progress>;
