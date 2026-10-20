use surt_engine::project::WaitingInput;

use super::{INDENT, answer_command, options};
use crate::message::Message;

pub(crate) fn block(subject: &str, waiting: &WaitingInput) -> Message {
    let headline = format!(
        "{} rev {} awaiting answers (round {})",
        waiting.artifact, waiting.revision, waiting.round
    );
    let questions = waiting.questions.questions().iter().enumerate();
    questions
        .fold(Message::new(headline), |message, (index, question)| {
            let message = message
                .context(format!("{}. {}", index + 1, question.question()))
                .context(format!("{INDENT}why: {}", question.why()));
            let message = match question.options() {
                Some(choices) => message.context(format!("{INDENT}options: {}", options(choices))),
                None => message,
            };
            message.context(format!("{INDENT}recommended: {}", question.recommended()))
        })
        .way("answer", answer_command(subject))
}
