pub(crate) use crate::runtime::FailureClass;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunId(pub(crate) String);

impl RunId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepRunId(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepKey(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ItemKey(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperationId(pub(crate) String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outcome(pub(crate) String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunState {
    Active,
    Succeeded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StepRunState {
    Pending,
    Running,
    Done(Outcome),
    Skipped,
    Blocked(BlockReason),
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockReason {
    AttemptsExhausted,
    NotRetryable,
    IterationsExhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionState {
    Queued,
    Running,
    Succeeded,
    Failed(FailureClass),
    Interrupted,
    Cancelled,
}

#[derive(Clone, Debug)]
pub(crate) struct RunGraph {
    pub(crate) root: WorkflowRun,
    pub(crate) cancel_requested: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowRun {
    pub(crate) id: RunId,
    pub(crate) state: RunState,
    pub(crate) steps: Vec<Step>,
}

#[derive(Clone, Debug)]
pub(crate) struct Step {
    pub(crate) id: StepRunId,
    pub(crate) key: StepKey,
    pub(crate) needs: Vec<StepKey>,
    pub(crate) when: Option<Condition>,
    pub(crate) state: StepRunState,
    pub(crate) work: Work,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Condition {
    pub(crate) step: StepKey,
    pub(crate) outcome: Outcome,
}

#[derive(Clone, Debug)]
pub(crate) enum Work {
    Agent(Attempts),
    Operation(OperationId, Attempts),
    Approval,
    Loop(Condition, Iterations),
    PerItem(Vec<Item>),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Attempts {
    pub(crate) executions: Vec<ExecutionState>,
    pub(crate) budget_start: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Iterations {
    pub(crate) bodies: Vec<Vec<Step>>,
    pub(crate) budget_start: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Item {
    pub(crate) key: ItemKey,
    pub(crate) needs: Vec<ItemKey>,
    pub(crate) run: WorkflowRun,
}
