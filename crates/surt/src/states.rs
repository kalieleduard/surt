use surt_engine::project::{ExecutionState, ItemState};

pub(crate) fn item_state_name(state: ItemState) -> &'static str {
    match state {
        ItemState::Pending => "pending",
        ItemState::Queued => "queued",
        ItemState::Running => "running",
        ItemState::Interrupted => "interrupted",
        ItemState::Blocked => "blocked",
        ItemState::Succeeded => "succeeded",
        ItemState::Cancelled => "cancelled",
    }
}

pub(crate) fn execution_state_name(state: ExecutionState) -> &'static str {
    match state {
        ExecutionState::Queued => "queued",
        ExecutionState::Running => "running",
        ExecutionState::Succeeded => "succeeded",
        ExecutionState::Failed => "failed",
        ExecutionState::Interrupted => "interrupted",
        ExecutionState::Cancelled => "cancelled",
    }
}
