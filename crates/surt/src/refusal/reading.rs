use surt_engine::project::KnownRun;

use crate::feature::quoted;
use crate::message::Message;
use crate::status::state_names;

pub(super) fn unknown_artifact(subject: &str, artifact: &str, known: &[String]) -> Message {
    let listed = if known.is_empty() {
        "it has no artifacts yet".to_owned()
    } else {
        format!("artifacts: {}", known.join(", "))
    };
    Message::new(format!("{subject} has no artifact {}", quoted(artifact)))
        .context(listed)
        .way("inspect", format!("surt show {subject}"))
}

pub(super) fn unknown_revision(
    subject: &str,
    artifact: &str,
    revision: u32,
    latest: u32,
) -> Message {
    Message::new(format!(
        "{artifact} has no rev {revision}; the latest is rev {latest}"
    ))
    .way(
        "inspect",
        format!("surt show {subject} {artifact} --rev {latest}"),
    )
}

pub(super) fn unknown_task(subject: &str, task: &str, known: &[String]) -> Message {
    let listed = if known.is_empty() {
        "it has no tasks yet".to_owned()
    } else {
        format!("tasks: {}", known.join(", "))
    };
    Message::new(format!("{subject} has no task {}", quoted(task)))
        .context(listed)
        .way("inspect", format!("surt status {subject}"))
}

pub(super) fn unknown_run(subject: &str, run: u32, runs: &[KnownRun]) -> Message {
    runs.iter()
        .fold(
            Message::new(format!("{subject} has no run {run}")),
            |message, known| {
                message.context(format!("run {}  {}", known.run, state_names(&known.states)))
            },
        )
        .way("inspect", format!("surt status {subject}"))
}
