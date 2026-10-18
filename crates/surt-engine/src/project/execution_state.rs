#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Interrupted,
    Cancelled,
}

impl ExecutionState {
    pub(crate) fn from_column(state: &str) -> ExecutionState {
        match state {
            "queued" => ExecutionState::Queued,
            "running" => ExecutionState::Running,
            "succeeded" => ExecutionState::Succeeded,
            "failed" => ExecutionState::Failed,
            "cancelled" => ExecutionState::Cancelled,
            _ => ExecutionState::Interrupted,
        }
    }
}
