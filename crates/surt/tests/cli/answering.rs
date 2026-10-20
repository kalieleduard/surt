use std::fs;

use jiff::Zoned;
use serde_json::Value;

use crate::scripts::{question, questions, spec, spec_restarted};
use crate::support::{Run, Sandbox};

const ACTOR: &str = "tester@example.com";

pub fn subject_name() -> String {
    format!("{}-add-login", Zoned::now().date().strftime("%Y%m%d"))
}

pub fn asked() -> Vec<Value> {
    vec![
        question("login-id", "e-mail only"),
        question("expiry", "7 days"),
    ]
}

pub fn asking(scripts: &[String]) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    let mut all = vec![questions(1, &asked())];
    all.extend_from_slice(scripts);
    sandbox.script(&all);
    ok(sandbox.run_feature("add login", &[]));
    sandbox
}

pub fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    let name = subject_name();
    let mut all = vec![args[0], name.as_str()];
    all.extend_from_slice(&args[1..]);
    sandbox.surt(&sandbox.repo(), &all)
}

pub fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

pub fn waiting_block(revision: u32, round: u32) -> String {
    let name = subject_name();
    format!(
        "? spec-questions rev {revision} awaiting answers (round {round})\n  \
         1. Which login-id?\n     \
         why: login-id changes the acceptance criteria\n     \
         recommended: e-mail only\n  \
         2. Which expiry?\n     \
         why: expiry changes the acceptance criteria\n     \
         recommended: 7 days\n  \
         answer: surt answer {name}\n"
    )
}

fn rounds(sandbox: &Sandbox) -> Value {
    let shown = surt(sandbox, &["show", "spec-answers"]);
    assert_eq!(shown.code, Some(0), "{}", shown.stderr);
    let document: Value = serde_json::from_str(&shown.stdout).expect("the answers are JSON");
    document["rounds"].clone()
}

fn answers(round: &Value) -> Value {
    round["answers"].clone()
}

fn pairs(expected: &[(&str, &str)]) -> Value {
    expected
        .iter()
        .map(|(id, answer)| ((*id).to_owned(), Value::from(*answer)))
        .collect::<serde_json::Map<_, _>>()
        .into()
}

#[test]
fn a_run_whose_spec_agent_asks_stops_with_the_questions_and_exit_0() {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[questions(1, &asked())]);

    let stderr = ok(sandbox.run_feature("add login", &[]));

    assert!(
        stderr.ends_with(&format!("✔ spec questions\n{}", waiting_block(1, 1))),
        "{stderr}"
    );
}

#[test]
fn accepting_continues_the_spec_conversation_to_the_gate_and_the_spec_is_then_approved() {
    let sandbox = asking(&[spec(2, "# Spec\n")]);
    let name = subject_name();

    let answered = ok(surt(&sandbox, &["answer", "--accept"]));
    let approved = ok(surt(&sandbox, &["approve"]));

    assert_eq!(
        answered,
        format!(
            "✔ answered spec-questions rev 1 by {ACTOR}\n\
             ● spec continued (iteration 2, same conversation)\n\
             ✔ spec rev 1 published\n\
             ✔ spec iter 2 done\n\
             ⏸ spec rev 1 awaiting approval in .surt/subjects/{name}\n  \
             approve: surt approve {name}\n  \
             reject: surt reject {name} -m \"<reason>\"\n"
        )
    );
    assert!(
        approved.starts_with("✔ approved spec rev 1 (agent)"),
        "{approved}"
    );
    let rounds = rounds(&sandbox);
    assert_eq!(rounds.as_array().map(Vec::len), Some(1));
    assert_eq!(
        answers(&rounds[0]),
        pairs(&[("expiry", "7 days"), ("login-id", "e-mail only")])
    );
    assert_eq!(rounds[0].get("note"), None);
}

#[test]
fn an_answers_file_records_the_answer_it_gives_each_question() {
    let sandbox = asking(&[spec(2, "# Spec\n")]);
    let file = sandbox.repo().join("answers.toml");
    fs::write(
        &file,
        "\"login-id\" = \"e-mail or username\"\nexpiry = \"30 days\"\n",
    )
    .expect("the answers file");

    let answered = ok(surt(&sandbox, &["answer", "--file", "answers.toml"]));

    assert!(
        answered.starts_with(&format!("✔ answered spec-questions rev 1 by {ACTOR}\n")),
        "{answered}"
    );
    assert_eq!(
        answers(&rounds(&sandbox)[0]),
        pairs(&[("expiry", "30 days"), ("login-id", "e-mail or username")])
    );
}

#[test]
fn a_message_accepts_the_recommendations_and_adds_the_note_to_the_round() {
    let sandbox = asking(&[spec(2, "# Spec\n")]);

    ok(surt(
        &sandbox,
        &["answer", "-m", "decide the rest yourself"],
    ));

    let round = &rounds(&sandbox)[0];
    assert_eq!(
        answers(round),
        pairs(&[("expiry", "7 days"), ("login-id", "e-mail only")])
    );
    assert_eq!(round["note"], "decide the rest yourself");
}

#[test]
fn a_second_round_of_questions_waits_again_on_the_next_revision() {
    let sandbox = asking(&[questions(2, &asked()), spec(3, "# Spec\n")]);

    let first = ok(surt(&sandbox, &["answer", "--accept"]));
    let second = ok(surt(&sandbox, &["answer", "--accept"]));

    assert!(
        first.ends_with(&format!("✔ spec iter 2 questions\n{}", waiting_block(2, 2))),
        "{first}"
    );
    assert!(
        second.starts_with(&format!("✔ answered spec-questions rev 2 by {ACTOR}\n")),
        "{second}"
    );
    assert_eq!(rounds(&sandbox).as_array().map(Vec::len), Some(2));
}

#[test]
fn a_conversation_that_cannot_be_continued_says_the_spec_restarted() {
    let sandbox = asking(&[spec_restarted(2, "# Spec\n")]);

    let answered = ok(surt(&sandbox, &["answer", "--accept"]));

    assert!(
        answered.contains(
            "● spec continued (iteration 2, same conversation)\n\
             ↻ spec restarted (iteration 2): transcript_missing\n"
        ),
        "{answered}"
    );
}
