use std::error::Error as StdError;
use std::io;
use std::process::ExitCode;

use surt_engine::project::Error;

use crate::message::Message;

const REFUSED: u8 = 3;
const INTERNAL: u8 = 1;

pub(crate) enum Failure {
    Refused(Message),
    Internal(Box<dyn StdError>),
}

impl Failure {
    pub(crate) fn report(self) -> ExitCode {
        match self {
            Failure::Refused(message) => {
                message.print_error();
                ExitCode::from(REFUSED)
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
