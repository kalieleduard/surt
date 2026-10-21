use surt_engine::project::WaitingInput;

use super::{INDENT, answer_command, options};
use crate::message::Message;

pub(super) fn headline(waiting: &WaitingInput) -> String {
    format!(
        "{} rev {} awaiting answers (round {})",
        waiting.artifact, waiting.revision, waiting.round
    )
}

pub(crate) fn block(subject: &str, waiting: &WaitingInput) -> Message {
    let headline = headline(waiting);
    let questions = waiting.questions.questions().iter().enumerate();
    questions
        .fold(Message::new(headline), |message, (index, question)| {
            let message = message
                .context(format!("{}. {}", index + 1, question.question()))
                .context(format!("{INDENT}why: {}", question.why()));
            let listed = question.options().map(options).unwrap_or_default();
            let message = listed.into_iter().fold(message, |message, line| {
                message.context(format!("{INDENT}{line}"))
            });
            message.context(format!("{INDENT}recommended: {}", question.recommended()))
        })
        .way("answer", answer_command(subject))
}
