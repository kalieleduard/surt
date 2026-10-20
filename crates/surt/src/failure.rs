use std::error::Error as StdError;
use std::io;
use std::process::ExitCode;

use surt_engine::project::Error;

use crate::message::{Mark, Message};

const INTERNAL: u8 = 1;
const USAGE: u8 = 2;
const REFUSED: u8 = 3;
const BLOCKED: u8 = 4;
const CANCELLED: u8 = 5;
const INTERRUPTED: u8 = 130;

pub(crate) enum Failure {
    Refused(Message),
    Blocked(Message),
    Cancelled(Message),
    Interrupted(Message),
    Incomplete(Message),
    Usage(clap::Error),
    Internal(Box<dyn StdError>),
}

impl Failure {
    pub(crate) fn report(self) -> ExitCode {
        match self {
            Failure::Refused(message) => stop(&message, Mark::Error, REFUSED),
            Failure::Blocked(message) => stop(&message, Mark::Blocked, BLOCKED),
            Failure::Cancelled(message) => stop(&message, Mark::Error, CANCELLED),
            Failure::Interrupted(message) => stop(&message, Mark::Stopped, INTERRUPTED),
            Failure::Incomplete(message) => stop(&message, Mark::Error, INTERNAL),
            Failure::Usage(error) => {
                let _ = error.print();
                ExitCode::from(USAGE)
            }
            Failure::Internal(error) => {
                Message::new(format!("internal error: {}", chain(error.as_ref())))
                    .context("rerun with SURT_LOG=debug for details")
                    .print_error();
                ExitCode::from(INTERNAL)
            }
        }
    }
}

fn stop(message: &Message, mark: Mark, code: u8) -> ExitCode {
    message.print(mark);
    ExitCode::from(code)
}

fn chain(error: &dyn StdError) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

impl From<Error> for Failure {
    fn from(error: Error) -> Failure {
        match error {
            Error::Refused(refusal) => Failure::Refused(crate::refusal::message(&refusal)),
            Error::Internal(internal) => Failure::Internal(Box::new(internal)),
        }
    }
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Failure {
        Failure::Internal(Box::new(error))
    }
}
