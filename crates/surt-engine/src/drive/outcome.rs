use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::cleanup::Cleanup;
use crate::project::Refusal;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriveOutcome {
    AtGate { gates: Vec<Gate> },
    Succeeded,
    Blocked { steps: Vec<StepLabel> },
    Cancelled,
    AlreadyCancelled,
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
    Merged {
        step: StepLabel,
        merge: String,
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
    EditRecorded {
        artifact: String,
        revision: u32,
    },
    EditSuperseded {
        artifact: String,
        edit: u32,
        by: u32,
    },
    FileRestored {
        artifact: String,
        path: String,
        revision: u32,
    },
    Recovering {
        driver: Option<u32>,
    },
    OrphanStopped {
        step: StepLabel,
        attempt: u32,
        pid: u32,
    },
    Discarded {
        step: StepLabel,
        attempt: u32,
    },
    InterruptedWithDriver {
        step: StepLabel,
        attempt: u32,
    },
    IntegrationRecorded {
        step: StepLabel,
        branch: String,
        merge: String,
    },
    CancelRequested {
        driver: Option<u32>,
    },
    StopRequested,
    /// Agent launches wait until `until` (Unix seconds) after a Transient
    /// Failure; `backoff` is the delay in seconds when no reset time was
    /// reported, and `waiting` counts the agent Step Runs ready to launch.
    Paused {
        until: i64,
        waiting: usize,
        backoff: Option<u64>,
    },
    Resumed {
        paused_seconds: u64,
    },
    CleanedUp(Cleanup),
    Activity(ActivitySnapshot),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivitySnapshot {
    pub running: Vec<RunningStep>,
    pub queued: usize,
    pub slots_in_use: usize,
    pub slots_limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningStep {
    pub step: StepLabel,
    pub started_at: i64,
}

pub type ProgressSink = UnboundedSender<Progress>;
