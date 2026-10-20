use std::fs;

use crate::answering::{asking, ok, subject_name, surt};
use crate::scripts::spec;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const USAGE_EXIT_CODE: i32 = 2;

fn refused(run: Run) -> String {
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

fn still_waiting(sandbox: &Sandbox) {
    let status = surt(sandbox, &["status"]);
    assert!(
        status.stdout.contains("AWAITING ANSWER"),
        "{}",
        status.stdout
    );
}

#[test]
fn the_answer_forms_are_mutually_exclusive() {
    let sandbox = asking(&[]);
    let combinations: [&[&str]; 3] = [
        &["--accept", "--file", "answers.toml"],
        &["--accept", "-m", "note"],
        &["--file", "answers.toml", "-m", "note"],
    ];

    for flags in combinations {
        let mut args = vec!["answer"];
        args.extend_from_slice(flags);

        let run = surt(&sandbox, &args);

        assert_eq!(run.code, Some(USAGE_EXIT_CODE), "{flags:?}: {}", run.stderr);
        assert!(run.stderr.contains("cannot be used with"), "{}", run.stderr);
    }
    still_waiting(&sandbox);
}

#[test]
fn without_a_terminal_and_without_a_flag_answer_is_a_usage_error() {
    let sandbox = asking(&[]);

    let run = surt(&sandbox, &["answer"]);

    assert_eq!(run.code, Some(USAGE_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with(
            "error: there is no terminal to ask the questions on; \
             answer with --accept, --file <path> or -m <text>\n"
        ),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("Usage: surt answer"), "{}", run.stderr);
    still_waiting(&sandbox);
}

#[test]
fn an_answers_file_that_misses_or_invents_ids_is_refused_listing_them() {
    let sandbox = asking(&[]);
    let name = subject_name();
    fs::write(
        sandbox.repo().join("answers.toml"),
        "\"login-id\" = \"e-mail only\"\ntypo = \"yes\"\n",
    )
    .expect("the answers file");

    let stderr = refused(surt(&sandbox, &["answer", "--file", "answers.toml"]));

    assert_eq!(
        stderr,
        format!(
            "✖ the answers do not match the questions of spec-questions rev 1\n  \
             unanswered: expiry\n  \
             unknown: typo\n  \
             inspect: surt show {name} spec-questions --rev 1\n"
        )
    );
    still_waiting(&sandbox);
}

#[test]
fn an_answers_file_that_is_missing_or_not_toml_is_refused_naming_it() {
    let sandbox = asking(&[]);
    fs::write(sandbox.repo().join("broken.toml"), "expiry = [\n").expect("the answers file");

    let missing = refused(surt(&sandbox, &["answer", "--file", "missing.toml"]));
    let broken = refused(surt(&sandbox, &["answer", "--file", "broken.toml"]));

    assert!(
        missing.starts_with("✖ cannot read the answers in missing.toml\n"),
        "{missing}"
    );
    assert!(
        broken.starts_with("✖ invalid answers in broken.toml:"),
        "{broken}"
    );
    still_waiting(&sandbox);
}

#[test]
fn answering_while_only_an_approval_waits_says_what_the_run_waits_for() {
    let sandbox = asking(&[spec(2, "# Spec\n")]);
    let name = subject_name();
    ok(surt(&sandbox, &["answer", "--accept"]));

    let stderr = refused(surt(&sandbox, &["answer", "--accept"]));

    assert_eq!(
        stderr,
        format!(
            "✖ {name} has no input step waiting (AWAITING APPROVAL)\n  \
             inspect: surt status {name}\n"
        )
    );
}

#[test]
fn approving_or_rejecting_while_only_the_questions_wait_is_refused() {
    let sandbox = asking(&[]);
    let name = subject_name();
    let expected = format!(
        "✖ {name} has no approval gate waiting\n  \
         answer-spec is waiting for answers, not for approval\n  \
         next: surt answer {name}\n"
    );

    let approved = refused(surt(&sandbox, &["approve"]));
    let rejected = refused(surt(&sandbox, &["reject", "-m", "shorter"]));

    assert_eq!(approved, expected);
    assert_eq!(rejected, expected);
    still_waiting(&sandbox);
}
