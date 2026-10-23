use std::fmt;
use std::io;
use std::process::ExitCode;

use clap::Parser;

use super::prompt::Edited;
use super::{SplitError, split_line};
use crate::message::Message;
use crate::{Cli, Command};

const PROGRAM: &str = "surt";

/// Shows each suggested command on the editable line and runs what the
/// developer submits, while the commands keep suggesting a next one. Returns
/// the exit code of the last command that ran.
pub(crate) async fn follow<N, P, E, F>(
    mut code: ExitCode,
    mut next: N,
    mut prompt: P,
    mut execute: E,
) -> ExitCode
where
    N: FnMut() -> Option<String>,
    P: FnMut(&str) -> io::Result<Edited>,
    E: FnMut(Command) -> F,
    F: Future<Output = ExitCode>,
{
    'follow: while let Some(suggested) = next() {
        let mut text = suggested;
        loop {
            let Ok(Edited::Submitted(line)) = prompt(&text) else {
                break 'follow;
            };
            match parse_line(&line) {
                Ok(Line::Empty) => break 'follow,
                Ok(Line::Command(cli)) => {
                    code = execute(cli.command).await;
                    break;
                }
                Err(error) => {
                    error.print();
                    text = line;
                }
            }
        }
    }
    code
}

pub(crate) enum Line {
    Empty,
    Command(Cli),
}

#[derive(Debug)]
pub(crate) enum LineError {
    Split(SplitError),
    NotSurt,
    Clap(clap::Error),
}

impl LineError {
    fn print(&self) {
        match self {
            LineError::Clap(error) => {
                let _ = error.print();
            }
            other => Message::new(other.to_string()).print_error(),
        }
    }
}

impl fmt::Display for LineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineError::Split(error) => error.fmt(formatter),
            LineError::NotSurt => write!(formatter, "only `{PROGRAM}` commands can be run here"),
            LineError::Clap(error) => error.fmt(formatter),
        }
    }
}

pub(crate) fn parse_line(line: &str) -> Result<Line, LineError> {
    let arguments = split_line(line).map_err(LineError::Split)?;
    match arguments.first() {
        None => Ok(Line::Empty),
        Some(program) if program == PROGRAM => Cli::try_parse_from(arguments)
            .map(Line::Command)
            .map_err(LineError::Clap),
        Some(_) => Err(LineError::NotSurt),
    }
}

#[cfg(test)]
mod tests;
