use surt_engine::project::ExecutionCandidate;

use crate::feature::quoted;
use crate::inspect::scope;
use crate::message::Message;
use crate::states::execution_state_name;
use crate::symbols::{ELLIPSIS, SEPARATOR};
use crate::table;

const LISTED: usize = 10;
const USE_MORE: &str = "use the full id, as surt prints it";

pub(super) fn unknown(reference: &str) -> Message {
    Message::new(format!(
        "no feature or execution matches {}",
        quoted(reference)
    ))
    .way("inspect", "surt status")
}

pub(super) fn too_short(
    reference: &str,
    minimum: usize,
    candidates: &[ExecutionCandidate],
) -> Message {
    listed(
        Message::new(format!(
            "{} is too short; an execution id prefix needs at least {minimum} characters",
            quoted(reference)
        )),
        candidates,
    )
}

pub(super) fn ambiguous(reference: &str, candidates: &[ExecutionCandidate]) -> Message {
    listed(
        Message::new(format!(
            "{} matches {} executions:",
            quoted(reference),
            candidates.len()
        )),
        candidates,
    )
}

fn listed(message: Message, candidates: &[ExecutionCandidate]) -> Message {
    let rows: Vec<[String; 3]> = candidates
        .iter()
        .take(LISTED)
        .map(|candidate| {
            [
                candidate.id.clone(),
                described(candidate),
                execution_state_name(candidate.state).to_owned(),
            ]
        })
        .collect();
    let message = table::render(&rows).lines().fold(message, |message, line| {
        message.context(format!("  {line}"))
    });
    let message = match candidates.len().checked_sub(LISTED) {
        Some(more) if more > 0 => message.context(format!("  {ELLIPSIS} and {more} more")),
        _ => message,
    };
    message.context(USE_MORE)
}

fn described(candidate: &ExecutionCandidate) -> String {
    let mut parts = vec![candidate.subject.clone(), format!("run {}", candidate.run)];
    parts.extend(scope(
        candidate.item.as_deref(),
        Some(&candidate.step),
        candidate.iteration,
        Some(candidate.attempt),
    ));
    parts.join(SEPARATOR)
}
