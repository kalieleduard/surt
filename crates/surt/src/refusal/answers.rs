use surt_engine::project::RunState;

use crate::answer::answer_command;
use crate::message::Message;
use crate::status::state_names;

pub(super) fn no_gate(subject: &str, input: Option<&str>) -> Message {
    let message = Message::new(format!("{subject} has no approval gate waiting"));
    match input {
        Some(step) => message
            .context(format!("{step} is waiting for answers, not for approval"))
            .way("next", answer_command(subject)),
        None => message.way("run", format!("surt status {subject}")),
    }
}

pub(super) fn no_input(subject: &str, states: &[RunState]) -> Message {
    Message::new(format!(
        "{subject} has no input step waiting ({})",
        state_names(states)
    ))
    .way("inspect", format!("surt status {subject}"))
}

pub(super) fn unanswered(
    subject: &str,
    artifact: &str,
    revision: u32,
    missing: &[String],
    unknown: &[String],
) -> Message {
    let listed = [("unanswered", missing), ("unknown", unknown)]
        .into_iter()
        .filter(|(_, ids)| !ids.is_empty())
        .fold(
            Message::new(format!(
                "the answers do not match the questions of {artifact} rev {revision}"
            )),
            |message, (label, ids)| message.context(format!("{label}: {}", ids.join(", "))),
        );
    listed.way(
        "inspect",
        format!("surt show {subject} {artifact} --rev {revision}"),
    )
}
