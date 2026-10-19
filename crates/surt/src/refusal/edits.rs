use crate::message::Message;

pub(super) fn unapproved(subject: &str, artifact: &str, revision: u32) -> Message {
    Message::new(format!(
        "{artifact} changed after approval (rev {revision} not approved)"
    ))
    .way("review it, then", format!("surt approve {subject}"))
}

pub(super) fn pinned(subject: &str, artifact: &str, revision: u32, files: &[String]) -> Message {
    Message::new(format!(
        "{} changed, but {artifact} rev {revision} is pinned (a later step already read it)",
        files.join(", ")
    ))
    .context("the edit was not recorded")
    .way("undo", restore(subject, artifact))
}

pub(super) fn invalid(subject: &str, artifact: &str, problem: &str) -> Message {
    Message::new(format!("{artifact} edit is invalid, no revision recorded:"))
        .context(format!("  {problem}"))
        .way("undo", restore(subject, artifact))
}

pub(super) fn not_restorable(subject: &str, artifact: &str, restorable: &[String]) -> Message {
    let listed = match restorable {
        [] => format!("{subject} has no artifact with a working copy"),
        names => format!("restorable: {}", names.join(", ")),
    };
    Message::new(format!("{artifact} has no working copy to restore"))
        .context(listed)
        .way("inspect", format!("surt status {subject}"))
}

pub(super) fn terminal_run(subject: &str, run: u32, files: &[String]) -> Message {
    let message = Message::new(format!(
        "{subject} run {run} has ended; restore only rewrites an active run"
    ));
    match files {
        [] => message.way("inspect", format!("surt status {subject}")),
        [_] => kept(message, subject, files).way("fix", "move or delete the file"),
        _ => kept(message, subject, files).way("fix", "move or delete the files"),
    }
}

fn kept(message: Message, subject: &str, files: &[String]) -> Message {
    message.context(format!("  {}", in_working_copy(subject, files).join(" ")))
}

pub(super) fn blocking_new_run(subject: &str, run: u32, files: &[String]) -> Message {
    let what = match files {
        [_] => "the file",
        _ => "the files",
    };
    Message::new(format!("working copy has unrecorded edits from run {run}"))
        .context(format!("  {}", in_working_copy(subject, files).join(" ")))
        .way("fix", format!("move or delete {what}, then run again"))
}

fn restore(subject: &str, artifact: &str) -> String {
    format!("surt restore {subject} {artifact}")
}

fn in_working_copy(subject: &str, files: &[String]) -> Vec<String> {
    files
        .iter()
        .map(|file| format!(".surt/subjects/{subject}/{file}"))
        .collect()
}
