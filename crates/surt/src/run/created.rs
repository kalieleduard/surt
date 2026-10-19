use surt_engine::project::{Commit, FreeName};

use crate::message::Message;

const SHORT_SHA_CHARS: usize = 7;
const FIELD_GAP: &str = "   ";

pub(crate) fn short_commit(commit: &Commit) -> String {
    let short = commit.sha.get(..SHORT_SHA_CHARS).unwrap_or(&commit.sha);
    match &commit.branch {
        Some(branch) => format!("{branch} @ {short}"),
        None => short.to_owned(),
    }
}

pub(super) fn line(name: &FreeName, workflow: &str, git_refs: &[(String, Commit)]) -> Message {
    let mut headline = format!("created {}{FIELD_GAP}workflow: {workflow}", name.name);
    for (input, commit) in git_refs {
        headline.push_str(&format!("{FIELD_GAP}{input}: {}", short_commit(commit)));
    }
    let message = Message::new(headline);
    if name.is_suffixed() {
        return message.context(format!("{} already exists", name.stem));
    }
    message
}
