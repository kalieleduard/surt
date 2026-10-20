use super::launcher::Launched;
use crate::runtime::{Conversation, Exit, RuntimeRequest, StopLevel, StopSignal};

pub(super) fn needed(
    request: &RuntimeRequest,
    continued: &Launched,
    stop: &StopSignal,
) -> Option<Conversation> {
    let continuation = request.continuation.as_ref()?;
    if continued.exit == Exit::Stopped || stop.level() != StopLevel::None {
        return None;
    }
    let reason = continued.transcript.refused_continuation()?;
    Some(Conversation::Restarted {
        reason,
        previous: continuation.execution_id.clone(),
    })
}

pub(super) fn fresh(request: &RuntimeRequest) -> RuntimeRequest {
    RuntimeRequest {
        continuation: None,
        ..request.clone()
    }
}
