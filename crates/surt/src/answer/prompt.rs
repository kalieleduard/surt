use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};

use surt_engine::project::{Question, Questions};

use super::{INDENT, answer_command, options};
use crate::message::Message;

pub(super) fn ask(
    questions: &Questions,
    mut input: impl BufRead,
    mut output: impl Write,
) -> io::Result<Option<BTreeMap<String, String>>> {
    let mut answers = BTreeMap::new();
    for (index, question) in questions.questions().iter().enumerate() {
        output.write_all(asked(index + 1, question).as_bytes())?;
        output.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let typed = line.trim();
        let answer = if typed.is_empty() {
            question.recommended()
        } else {
            typed
        };
        answers.insert(question.id().to_owned(), answer.to_owned());
    }
    Ok(Some(answers))
}

pub(super) fn ended(subject: &str) -> Message {
    Message::new("the input ended before every question was answered")
        .way("or", format!("{} --accept", answer_command(subject)))
}

fn asked(number: usize, question: &Question) -> String {
    let mut text = format!(
        "{number}. {}\n{INDENT}why: {}\n",
        question.question(),
        question.why()
    );
    if let Some(choices) = question.options() {
        text.push_str(&format!("{INDENT}options: {}\n", options(choices)));
    }
    text.push_str(&format!("{INDENT}[{}] ", question.recommended()));
    text
}
