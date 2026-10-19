use std::fs;
use std::io;
use std::path::Path;

use crate::failure::Failure;
use crate::message::Message;
use crate::retry::word;

const FILE_PREFIX: char = '@';
const SEPARATOR: char = '=';

#[derive(Clone, Debug)]
pub(crate) struct InputArg {
    pub(crate) name: String,
    pub(crate) raw: String,
}

pub(crate) fn parse(text: &str) -> Result<InputArg, String> {
    match text.split_once(SEPARATOR) {
        Some((name, raw)) if !name.is_empty() => Ok(InputArg {
            name: name.to_owned(),
            raw: raw.to_owned(),
        }),
        _ => Err(format!(
            "expected <name>{SEPARATOR}<value> or <name>{SEPARATOR}{FILE_PREFIX}<path>"
        )),
    }
}

pub(crate) fn read(cwd: &Path, inputs: &[InputArg]) -> Result<Vec<(String, String)>, Failure> {
    inputs
        .iter()
        .map(|input| Ok((input.name.clone(), value(cwd, input)?)))
        .collect()
}

impl InputArg {
    pub(crate) fn rendered(&self) -> String {
        format!("{}{SEPARATOR}{}", self.name, self.raw)
    }

    pub(crate) fn quoted(&self) -> String {
        format!("{}{SEPARATOR}{}", self.name, word(&self.raw))
    }
}

fn value(cwd: &Path, input: &InputArg) -> Result<String, Failure> {
    let Some(path) = input.raw.strip_prefix(FILE_PREFIX) else {
        return Ok(input.raw.clone());
    };
    let bytes = fs::read(cwd.join(path)).map_err(|error| unreadable(input, &error))?;
    String::from_utf8(bytes).map_err(|_| {
        Failure::Refused(
            Message::new(format!("--input {} is not UTF-8 text", input.rendered()))
                .way("fix", format!("save {path} as UTF-8")),
        )
    })
}

fn unreadable(input: &InputArg, error: &io::Error) -> Failure {
    Failure::Refused(
        Message::new(format!("cannot read --input {}: {error}", input.rendered())).context(
            format!("{FILE_PREFIX}<path> is read relative to the current directory"),
        ),
    )
}
