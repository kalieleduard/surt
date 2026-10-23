use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::process::ExitCode;

use clap::CommandFactory;

use super::{Line, LineError, follow, parse_line};
use crate::follow::prompt::Edited;
use crate::follow::split_line;
use crate::{Cli, Command};

fn command(line: &str) -> Command {
    match parse_line(line) {
        Ok(Line::Command(cli)) => cli.command,
        Ok(Line::Empty) => panic!("`{line}` parsed as an empty line"),
        Err(error) => panic!("`{line}` was refused: {error}"),
    }
}

fn name(command: &Command) -> &'static str {
    match command {
        Command::Approve(_) => "approve",
        Command::Reject(_) => "reject",
        Command::Continue(_) => "continue",
        _ => "other",
    }
}

#[test]
fn a_line_not_starting_with_surt_is_refused() {
    assert!(matches!(parse_line("ls -la"), Err(LineError::NotSurt)));
    assert!(matches!(
        parse_line("cargo surt approve s"),
        Err(LineError::NotSurt)
    ));
}

#[test]
fn the_refusal_names_surt() {
    assert_eq!(
        LineError::NotSurt.to_string(),
        "only `surt` commands can be run here"
    );
}

#[test]
fn an_empty_line_is_empty() {
    assert!(matches!(parse_line(""), Ok(Line::Empty)));
    assert!(matches!(parse_line("   \t "), Ok(Line::Empty)));
}

#[test]
fn an_unterminated_quote_is_refused() {
    assert!(matches!(
        parse_line("surt reject s -m \"motivo"),
        Err(LineError::Split(_))
    ));
}

#[test]
fn arguments_clap_refuses_are_refused() {
    assert!(matches!(
        parse_line("surt reject s"),
        Err(LineError::Clap(_))
    ));
    assert!(matches!(
        parse_line("surt nonsense"),
        Err(LineError::Clap(_))
    ));
    assert!(matches!(parse_line("surt"), Err(LineError::Clap(_))));
}

#[test]
fn approve_parses() {
    assert_eq!(name(&command("surt approve s")), "approve");
}

#[test]
fn reject_with_a_quoted_reason_parses() {
    let line = "surt reject s -m \"motivo com espaços\"";
    assert_eq!(name(&command(line)), "reject");
    let matches = Cli::command()
        .try_get_matches_from(split_line(line).unwrap())
        .unwrap();
    let reject = matches.subcommand_matches("reject").unwrap();
    assert_eq!(
        reject.get_one::<String>("reason").map(String::as_str),
        Some("motivo com espaços")
    );
}

/// Drives `follow` with scripted prompt answers. Each executed command
/// returns the next code and may suggest the next command.
struct Script {
    suggestions: RefCell<VecDeque<String>>,
    answers: RefCell<VecDeque<io::Result<Edited>>>,
    shown: RefCell<Vec<String>>,
    executed: RefCell<Vec<&'static str>>,
    results: RefCell<VecDeque<(u8, Option<&'static str>)>>,
}

impl Script {
    fn new(
        first: &str,
        answers: Vec<io::Result<Edited>>,
        results: Vec<(u8, Option<&'static str>)>,
    ) -> Script {
        Script {
            suggestions: RefCell::new(VecDeque::from([first.to_owned()])),
            answers: RefCell::new(answers.into()),
            shown: RefCell::default(),
            executed: RefCell::default(),
            results: RefCell::new(results.into()),
        }
    }

    async fn run(&self, code: u8) -> ExitCode {
        follow(
            ExitCode::from(code),
            || self.suggestions.borrow_mut().pop_front(),
            |text| {
                self.shown.borrow_mut().push(text.to_owned());
                self.answers.borrow_mut().pop_front().unwrap()
            },
            |command| {
                self.executed.borrow_mut().push(name(&command));
                let (code, next) = self.results.borrow_mut().pop_front().unwrap();
                self.suggestions
                    .borrow_mut()
                    .extend(next.map(str::to_owned));
                async move { ExitCode::from(code) }
            },
        )
        .await
    }
}

fn submit(line: &str) -> io::Result<Edited> {
    Ok(Edited::Submitted(line.to_owned()))
}

#[tokio::test]
async fn submitted_commands_chain_until_the_developer_quits() {
    let script = Script::new(
        "surt approve s",
        vec![
            submit("surt approve s"),
            submit("surt continue s"),
            Ok(Edited::Quit),
        ],
        vec![(0, Some("surt continue s")), (4, Some("surt continue s"))],
    );
    assert_eq!(script.run(0).await, ExitCode::from(4));
    assert_eq!(*script.executed.borrow(), ["approve", "continue"]);
    assert_eq!(
        *script.shown.borrow(),
        ["surt approve s", "surt continue s", "surt continue s"]
    );
}

#[tokio::test]
async fn chain_ends_when_a_command_suggests_nothing() {
    let script = Script::new(
        "surt continue s",
        vec![submit("surt continue s")],
        vec![(3, None)],
    );
    assert_eq!(script.run(130).await, ExitCode::from(3));
    assert_eq!(*script.executed.borrow(), ["continue"]);
}

#[tokio::test]
async fn quitting_keeps_the_code() {
    let script = Script::new("surt continue s", vec![Ok(Edited::Quit)], vec![]);
    assert_eq!(script.run(130).await, ExitCode::from(130));
    assert!(script.executed.borrow().is_empty());
}

#[tokio::test]
async fn an_empty_line_quits() {
    let script = Script::new("surt continue s", vec![submit("  ")], vec![]);
    assert_eq!(script.run(4).await, ExitCode::from(4));
    assert!(script.executed.borrow().is_empty());
}

#[tokio::test]
async fn a_prompt_error_stops_following() {
    let script = Script::new(
        "surt continue s",
        vec![Err(io::Error::other("no raw mode"))],
        vec![],
    );
    assert_eq!(script.run(4).await, ExitCode::from(4));
    assert!(script.executed.borrow().is_empty());
}

#[tokio::test]
async fn a_refused_line_is_shown_again_as_typed() {
    let script = Script::new(
        "surt approve s",
        vec![
            submit("ls"),
            submit("surt reject s"),
            submit("surt reject s -m 'motivo'"),
        ],
        vec![(0, None)],
    );
    assert_eq!(script.run(0).await, ExitCode::SUCCESS);
    assert_eq!(*script.executed.borrow(), ["reject"]);
    assert_eq!(
        *script.shown.borrow(),
        ["surt approve s", "ls", "surt reject s"]
    );
}
