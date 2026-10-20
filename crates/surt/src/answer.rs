mod file;
mod prompt;
mod waiting;

use std::collections::BTreeMap;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::{Answered, Project, Questions, WaitingInput};

use crate::continue_run::{active_run, drive_again};
use crate::failure::Failure;
use crate::message::Message;
use crate::symbols::{SEPARATOR, printable};
pub(crate) use waiting::block as waiting_block;

const INDENT: &str = "   ";
const NO_TERMINAL: &str = "there is no terminal to ask the questions on; answer with --accept, --file <path> or -m <text>";

#[derive(clap::Args)]
pub(crate) struct AnswerArgs {
    subject: String,
    #[arg(long, conflicts_with_all = ["file", "note"])]
    accept: bool,
    #[arg(long, value_name = "path", conflicts_with = "note")]
    file: Option<PathBuf>,
    #[arg(short = 'm', long = "message", value_name = "text", value_parser = NonEmptyStringValueParser::new())]
    note: Option<String>,
}

enum Source {
    Accepted,
    Noted(String),
    File(PathBuf),
    Terminal,
}

pub(crate) fn answer_command(subject: &str) -> String {
    format!("surt answer {subject}")
}

pub(crate) async fn run(cwd: &Path, args: AnswerArgs) -> Result<(), Failure> {
    let source = source(args.accept, args.file, args.note)?;
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.subject).await?;
    let run = active_run(&subject)?;
    let waiting = project.waiting_input(run).await?;
    let (answers, note) = answers(cwd, &subject.name, &waiting, source)?;
    let actor = project.actor().await?;
    let answered = project
        .answer(run, waiting.revision, &answers, note.as_deref(), &actor)
        .await?;
    recorded(&answered).print_success();
    drive_again(&project, &subject).await
}

fn source(accept: bool, file: Option<PathBuf>, note: Option<String>) -> Result<Source, Failure> {
    if accept {
        return Ok(Source::Accepted);
    }
    if let Some(path) = file {
        return Ok(Source::File(path));
    }
    if let Some(note) = note {
        return Ok(Source::Noted(note));
    }
    if io::stdin().is_terminal() {
        return Ok(Source::Terminal);
    }
    Err(Failure::Usage(usage_error()))
}

fn usage_error() -> clap::Error {
    let mut command = <crate::Cli as clap::CommandFactory>::command();
    command.build();
    match command.find_subcommand_mut("answer") {
        Some(answer) => answer.error(clap::error::ErrorKind::MissingRequiredArgument, NO_TERMINAL),
        None => command.error(clap::error::ErrorKind::MissingRequiredArgument, NO_TERMINAL),
    }
}

fn answers(
    cwd: &Path,
    subject: &str,
    waiting: &WaitingInput,
    source: Source,
) -> Result<(BTreeMap<String, String>, Option<String>), Failure> {
    match source {
        Source::Accepted => Ok((recommended(&waiting.questions), None)),
        Source::Noted(note) => Ok((recommended(&waiting.questions), Some(note))),
        Source::File(path) => {
            let inspect = format!(
                "surt show {subject} {} --rev {}",
                waiting.artifact, waiting.revision
            );
            Ok((file::read(&cwd.join(&path), &path, inspect)?, None))
        }
        Source::Terminal => {
            let asked = prompt::ask(&waiting.questions, io::stdin().lock(), Printable)?;
            asked
                .map(|answers| (answers, None))
                .ok_or_else(|| Failure::Refused(prompt::ended(subject)))
        }
    }
}

fn recommended(questions: &Questions) -> BTreeMap<String, String> {
    questions
        .questions()
        .iter()
        .map(|question| (question.id().to_owned(), question.recommended().to_owned()))
        .collect()
}

fn recorded(answered: &Answered) -> Message {
    Message::new(format!(
        "answered {} rev {} by {}",
        answered.artifact, answered.revision, answered.actor
    ))
}

fn options(choices: &[String]) -> String {
    choices.join(SEPARATOR)
}

struct Printable;

impl io::Write for Printable {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(bytes);
        io::stderr().write_all(printable(&text).as_bytes())?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stderr().flush()
    }
}

#[cfg(test)]
mod tests;
