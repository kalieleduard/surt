use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::cleanup::Cleanup;
use crate::project::Refusal;
use crate::runtime::RestartReason;

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
    pub kind: GateKind,
    pub artifact: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateKind {
    Approval,
    Input,
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
        continued: bool,
    },
    ConversationRestarted {
        step: StepLabel,
        reason: RestartReason,
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
    ConfigInvalid {
        path: PathBuf,
        line: Option<usize>,
        reason: String,
        limit: usize,
    },
    Paused {
        pause: ActivityPause,
        waiting: usize,
    },
    Resumed {
        paused_seconds: i64,
    },
    CleanedUp(Cleanup),
    RunKept,
    Activity(ActivitySnapshot),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivitySnapshot {
    pub running: Vec<RunningStep>,
    pub queued: usize,
    pub slots_in_use: usize,
    pub slots_limit: usize,
    pub paused: Option<ActivityPause>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivityPause {
    pub until: i64,
    pub backoff: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningStep {
    pub step: StepLabel,
    pub started_at: i64,
}

pub type ProgressSink = UnboundedSender<Progress>;
