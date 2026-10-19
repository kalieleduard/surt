use std::io::{self, BufRead, IsTerminal, Write};

use crate::failure::Failure;
use crate::message::Message;
use crate::symbols::printable;

const PROMPT: &str = "[y/N] ";
const ACCEPTED: [&str; 2] = ["y", "yes"];

pub(crate) fn confirmed(command: &str, retry: String, question: &str) -> Result<bool, Failure> {
    if !io::stdin().is_terminal() {
        return Err(Failure::Refused(
            Message::new(format!(
                "{command} asks for a confirmation and there is no terminal to ask"
            ))
            .way("fix", retry),
        ));
    }
    Ok(ask(&printable(question), io::stdin().lock(), io::stderr())?)
}

pub(crate) fn ask(
    question: &str,
    mut input: impl BufRead,
    mut output: impl Write,
) -> io::Result<bool> {
    write!(output, "{question}{PROMPT}")?;
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    Ok(ACCEPTED.contains(&answer.as_str()))
}
