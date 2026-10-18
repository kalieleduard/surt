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
    .way("fix", remove(subject, files))
}

pub(super) fn invalid(subject: &str, artifact: &str, problem: &str, files: &[String]) -> Message {
    Message::new(format!("{artifact} edit is invalid, no revision recorded:"))
        .context(format!("  {problem}"))
        .way("fix", remove(subject, files))
}

fn remove(subject: &str, files: &[String]) -> String {
    let paths: Vec<String> = files
        .iter()
        .map(|file| format!(".surt/features/{subject}/{file}"))
        .collect();
    format!("rm {}, then rerun", paths.join(" "))
}
