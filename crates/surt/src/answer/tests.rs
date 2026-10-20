use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use surt_engine::project::{Questions, WaitingInput};
use tempfile::TempDir;

use super::{file, prompt, waiting_block};
use crate::failure::Failure;
use crate::message::Mark;

const SUBJECT: &str = "20261020-add-login";
const INSPECT: &str = "surt show 20261020-add-login spec-questions --rev 1";

fn questions() -> Questions {
    let document = serde_json::json!({ "questions": [
        {
            "id": "login-id",
            "question": "Log in by e-mail only, or by username too?",
            "why": "changes the users table",
            "recommended": "e-mail only",
            "options": ["e-mail only", "e-mail or username"],
        },
        {
            "id": "expiry",
            "question": "Does a session expire, and after how long?",
            "why": "decides whether the Feature needs a refresh flow",
            "recommended": "7 days, no refresh",
        },
    ]});
    Questions::parse(document.to_string().as_bytes()).expect("valid questions")
}

fn owned(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(id, answer)| ((*id).to_owned(), (*answer).to_owned()))
        .collect()
}

fn refusal(result: Result<BTreeMap<String, String>, Failure>) -> String {
    match result {
        Err(Failure::Refused(message)) => message.plain_error(),
        Err(_) => panic!("not a refusal"),
        Ok(answers) => panic!("read {answers:?}"),
    }
}

fn answers_file(text: &str) -> (TempDir, std::path::PathBuf) {
    let directory = TempDir::new().expect("a temporary directory");
    let path = directory.path().join("answers.toml");
    fs::write(&path, text).expect("the answers file");
    (directory, path)
}

#[test]
fn the_prompt_asks_each_question_and_an_empty_line_takes_the_recommendation() {
    let mut output = Vec::new();

    let answers = prompt::ask(&questions(), "\n30 days\n".as_bytes(), &mut output);

    assert_eq!(
        answers.expect("the prompt reads"),
        Some(owned(&[("login-id", "e-mail only"), ("expiry", "30 days")]))
    );
    assert_eq!(
        String::from_utf8(output).expect("UTF-8"),
        "1. Log in by e-mail only, or by username too?\n   \
         why: changes the users table\n   \
         options: e-mail only · e-mail or username\n   \
         [e-mail only] \
         2. Does a session expire, and after how long?\n   \
         why: decides whether the Feature needs a refresh flow\n   \
         [7 days, no refresh] "
    );
}

#[test]
fn input_that_ends_before_the_last_question_answers_nothing() {
    let answers = prompt::ask(&questions(), "e-mail or username\n".as_bytes(), Vec::new());

    assert_eq!(answers.expect("the prompt reads"), None);
    assert_eq!(
        prompt::ended(SUBJECT).plain_error(),
        "✖ the input ended before every question was answered\n  \
         or: surt answer 20261020-add-login --accept\n"
    );
}

#[test]
fn the_waiting_block_lists_every_question_with_why_options_and_recommendation() {
    let waiting = WaitingInput {
        step: "answer-spec".to_owned(),
        artifact: "spec-questions".to_owned(),
        revision: 1,
        round: 2,
        questions: questions(),
    };

    assert_eq!(
        waiting_block(SUBJECT, &waiting).plain(Mark::Question),
        "? spec-questions rev 1 awaiting answers (round 2)\n  \
         1. Log in by e-mail only, or by username too?\n     \
         why: changes the users table\n     \
         options: e-mail only · e-mail or username\n     \
         recommended: e-mail only\n  \
         2. Does a session expire, and after how long?\n     \
         why: decides whether the Feature needs a refresh flow\n     \
         recommended: 7 days, no refresh\n  \
         answer: surt answer 20261020-add-login\n"
    );
}

#[test]
fn an_answers_file_maps_each_question_id_to_its_answer() {
    let (_directory, path) =
        answers_file("\"login-id\" = \"e-mail or username\"\nexpiry = \"never\"\n");

    let answers = file::read(&path, Path::new("answers.toml"), INSPECT.to_owned());

    assert_eq!(
        answers.ok(),
        Some(owned(&[
            ("login-id", "e-mail or username"),
            ("expiry", "never")
        ]))
    );
}

#[test]
fn an_answers_file_that_is_not_a_table_of_strings_is_refused_with_its_line() {
    let (_directory, path) = answers_file("\"login-id\" = \"e-mail only\"\nexpiry = 7\n");

    let refused = refusal(file::read(
        &path,
        Path::new("answers.toml"),
        INSPECT.to_owned(),
    ));

    assert!(
        refused.starts_with("✖ invalid answers in answers.toml:2\n  "),
        "{refused}"
    );
    assert!(
        refused.ends_with(
            "\n  expected one line per question: \"<id>\" = \"<answer>\"\n  \
             inspect: surt show 20261020-add-login spec-questions --rev 1\n"
        ),
        "{refused}"
    );
}

#[test]
fn an_answers_file_that_cannot_be_read_is_refused_naming_it() {
    let directory = TempDir::new().expect("a temporary directory");

    let refused = refusal(file::read(
        &directory.path().join("missing.toml"),
        Path::new("missing.toml"),
        INSPECT.to_owned(),
    ));

    assert!(
        refused.starts_with("✖ cannot read the answers in missing.toml\n  "),
        "{refused}"
    );
}
