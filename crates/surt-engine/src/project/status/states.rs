use crate::project::RunState;
use crate::store::WaitingGate;

const SUCCEEDED: &str = "succeeded";
const CANCELLED: &str = "cancelled";

pub(super) fn derive(
    stored: &str,
    driven: bool,
    waiting: &[WaitingGate],
    blocked: bool,
) -> Vec<RunState> {
    match stored {
        SUCCEEDED => return vec![RunState::Succeeded],
        CANCELLED => return vec![RunState::Cancelled],
        _ if driven => return vec![RunState::Running],
        _ => {}
    }
    let mut states = Vec::new();
    if waiting.iter().any(|gate| !gate.is_input()) {
        states.push(RunState::AwaitingApproval);
    }
    if waiting.iter().any(WaitingGate::is_input) {
        states.push(RunState::AwaitingAnswer);
    }
    if blocked {
        states.push(RunState::Blocked);
    }
    if states.is_empty() {
        states.push(RunState::Interrupted);
    }
    states
}
