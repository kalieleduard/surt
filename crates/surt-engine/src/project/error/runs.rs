use crate::project::ExecutionState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    AwaitingApproval,
    AwaitingAnswer,
    Blocked,
    Interrupted,
    Succeeded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub states: Vec<RunState>,
    pub awaiting: Option<AwaitedRevision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownRun {
    pub run: u32,
    pub states: Vec<RunState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionCandidate {
    pub id: String,
    pub subject: String,
    pub run: u32,
    pub item: Option<String>,
    pub step: String,
    pub iteration: Option<u32>,
    pub attempt: u32,
    pub state: ExecutionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AwaitedRevision {
    pub artifact: String,
    pub revision: u32,
}
