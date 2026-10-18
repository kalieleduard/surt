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

pub(super) fn no_revision_in_iteration(
    subject: &str,
    item: Option<&str>,
    artifact: &str,
    iteration: u32,
    iterations: &[u32],
) -> Message {
    let producer = item.map_or_else(|| subject.to_owned(), |item| format!("task {item}"));
    let task = item
        .map(|item| format!(" --task {item}"))
        .unwrap_or_default();
    let latest = iterations
        .last()
        .map(|last| format!(" --iter {last}"))
        .unwrap_or_default();
    Message::new(format!(
        "{producer} produced no {artifact} in iter {iteration}"
    ))
    .context(produced_in(artifact, iterations))
    .way(
        "inspect",
        format!("surt show {subject} {artifact}{task}{latest}"),
    )
}

fn produced_in(artifact: &str, iterations: &[u32]) -> String {
    if iterations.is_empty() {
        return format!("no iter produced {artifact}");
    }
    let numbers: Vec<String> = iterations.iter().map(u32::to_string).collect();
    format!("iters with {artifact}: {}", numbers.join(", "))
}

pub(super) fn unknown_file(artifact: &str, revision: u32, path: &str, files: &[String]) -> Message {
    let listed = if files.is_empty() {
        "it has no files".to_owned()
    } else {
        format!("files: {}", files.join(", "))
    };
    Message::new(format!(
        "{artifact} rev {revision} has no file {}",
        quoted(path)
    ))
    .context(listed)
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
