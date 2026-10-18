use std::io::Cursor;

use surt_engine::project::{CancelImpact, Cancellation};

use super::confirmation::{ask, question};
use super::ended;
use crate::message::Mark;

const SUBJECT: &str = "20261017-add-login";

fn impact(executions: usize, tasks: usize, merged: usize, branch: bool) -> CancelImpact {
    CancelImpact {
        executions,
        tasks,
        merged,
        integration_branch: branch.then(|| format!("surt/{SUBJECT}")),
    }
}

fn answered(answer: &str) -> (bool, String) {
    let mut asked = Vec::new();
    let accepted = ask(
        "cancel it?\n",
        Cursor::new(answer.as_bytes().to_vec()),
        &mut asked,
    )
    .expect("the answer is read");
    (accepted, String::from_utf8(asked).expect("utf-8"))
}

#[test]
fn the_question_shows_what_is_stopped_and_what_stays() {
    assert_eq!(
        question(SUBJECT, &impact(2, 4, 2, true)),
        "cancel 20261017-add-login?\n\
         \x20 2 running executions will be stopped · 2/4 tasks merged stay on surt/20261017-add-login\n\
         \x20 worktrees are removed; task branches and salvage refs stay\n\
         \x20 starting over later needs a new feature: surt feature \"<description>\"\n"
    );
}

#[test]
fn the_question_names_a_single_execution_and_a_run_without_tasks() {
    assert_eq!(
        question(SUBJECT, &impact(1, 0, 0, false)),
        "cancel 20261017-add-login?\n\
         \x20 1 running execution will be stopped\n\
         \x20 worktrees are removed; task branches and salvage refs stay\n\
         \x20 starting over later needs a new feature: surt feature \"<description>\"\n"
    );
    assert!(
        question(SUBJECT, &impact(0, 2, 0, true))
            .contains("  nothing is running · 0/2 tasks merged stay on surt/20261017-add-login\n")
    );
}

#[test]
fn only_yes_confirms() {
    for (answer, accepted) in [
        ("y\n", true),
        ("YES\n", true),
        (" yes \n", true),
        ("\n", false),
        ("n\n", false),
        ("maybe\n", false),
        ("", false),
    ] {
        let (confirmed, asked) = answered(answer);
        assert_eq!(confirmed, accepted, "{answer:?}");
        assert_eq!(asked, "cancel it?\n[y/N] ");
    }
}

#[test]
fn the_ending_says_whether_this_cancel_ended_the_run() {
    let lines: Vec<String> = [
        Cancellation::Cancelled,
        Cancellation::AlreadyCancelled,
        Cancellation::AlreadySucceeded,
    ]
    .into_iter()
    .map(|cancellation| ended(SUBJECT, cancellation).plain(Mark::Success))
    .collect();

    assert_eq!(
        lines,
        [
            "✔ run cancelled\n",
            "✔ 20261017-add-login was already cancelled\n",
            "✔ 20261017-add-login had already succeeded — nothing to cancel\n",
        ]
    );
}
