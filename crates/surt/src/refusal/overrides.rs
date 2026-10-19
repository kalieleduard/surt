use surt_engine::project::{InvalidOverride, Problem};

use super::definition;
use crate::message::Message;

pub(super) fn invalid_override(invalid: &InvalidOverride) -> Message {
    let file = &invalid.file;
    let reason = reason(invalid);
    let headline = match &invalid.field {
        Some(field) => format!("{file}: {field}: {reason}"),
        None => format!("{file}: {reason}"),
    };
    let message = match &invalid.needed_by {
        Some(needed) => Message::new(headline).context(format!(
            "needed by: {} step {}",
            needed.workflow, needed.step
        )),
        None => Message::new(headline),
    };
    match invalid.problem {
        Problem::StrayAgentFile | Problem::UnknownAgentDirectory { .. } => {
            message.way("fix", format!("rename or remove {file}"))
        }
        _ => message.way("fix", format!("edit {file}")),
    }
}

fn reason(invalid: &InvalidOverride) -> String {
    match (&invalid.needed_by, &invalid.problem) {
        (Some(_), Problem::UnknownOutcome { outcome, .. }) => {
            format!("outcomes no longer include \"{outcome}\"")
        }
        (_, problem) => definition::describe(problem),
    }
}
