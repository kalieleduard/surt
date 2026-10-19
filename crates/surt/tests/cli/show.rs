use std::fs;

use crate::delivering::{planned, subject_name};
use crate::scripts::{build, review, ship, spec, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SPEC_1: &str = "# Spec 1\n";
const SPEC_2: &str = "# Spec 2\nwith OAuth\n";
const USAGE_ERROR_EXIT_CODE: i32 = 2;

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> Run {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run
}

fn refused(run: Run) -> String {
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    run.stderr
}

fn with_two_specs(name: &str) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[spec(1, SPEC_1), spec(2, SPEC_2), tasks(1, &["A", "B"])]);
    ok(sandbox.run_feature("add login", &[]));
    ok(surt(
        &sandbox,
        &["reject", name, "-m", "falta o fluxo OAuth"],
    ));
    ok(surt(&sandbox, &["continue", name]));
    ok(surt(&sandbox, &["approve", name]));
    ok(surt(&sandbox, &["continue", name]));
    sandbox
}

#[test]
fn show_lists_the_artifacts_of_a_subject_with_their_latest_revision() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let run = ok(surt(&sandbox, &["show", "add-login"]));

    assert_eq!(
        run.stdout,
        "ARTIFACT               REVS  LATEST\n\
         request                1     rev 1 (human)\n\
         base                   1     rev 1 (human)\n\
         spec                   2     rev 2 (agent) ✔ approved\n\
         spec-rejection-reason  1     rev 1 (human)\n\
         manifest               1     rev 1 (agent) · 3 files\n"
    );
}

#[test]
fn run_1_of_a_single_run_subject_is_the_run_shown_by_default() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let spec = ok(surt(&sandbox, &["show", &name, "spec", "--run", "1"]));
    let index = ok(surt(&sandbox, &["show", &name, "--run", "1"]));

    assert_eq!(spec.stdout, SPEC_2);
    assert_eq!(index.stdout, ok(surt(&sandbox, &["show", &name])).stdout);
}

#[test]
fn an_unknown_run_is_refused_listing_the_runs_of_the_subject() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let by_show = refused(surt(&sandbox, &["show", &name, "spec", "--run", "2"]));
    let by_index = refused(surt(&sandbox, &["show", &name, "--run", "2"]));
    let by_status = refused(surt(&sandbox, &["status", &name, "--run", "2"]));

    let expected = format!(
        "✖ {name} has no run 2\n  \
         run 1  AWAITING APPROVAL\n  \
         inspect: surt status {name}\n"
    );
    assert_eq!(by_show, expected);
    assert_eq!(by_index, expected);
    assert_eq!(by_status, expected);
}

#[test]
fn show_prints_any_revision_byte_for_byte_on_stdout_and_nothing_else() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let latest = ok(surt(&sandbox, &["show", &name, "spec"]));
    let first = ok(surt(&sandbox, &["show", &name, "spec", "--rev", "1"]));
    let manifest = ok(surt(&sandbox, &["show", &name, "manifest"]));

    assert_eq!(latest.stdout, SPEC_2);
    assert_eq!(first.stdout, SPEC_1);
    assert!(latest.stderr.is_empty(), "{}", latest.stderr);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    assert_eq!(
        manifest.stdout,
        fs::read_to_string(working_copy.join("manifest.json")).expect("the manifest")
    );
}

#[test]
fn show_warns_on_stderr_when_the_working_copy_holds_an_unrecorded_edit() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);
    let working_copy = sandbox.dot_surt().join("features").join(&name);
    fs::write(working_copy.join("spec.md"), "# Spec, edited by hand\n").expect("an edit");

    let run = ok(surt(&sandbox, &["show", &name, "spec"]));

    assert_eq!(run.stdout, SPEC_2);
    assert_eq!(
        run.stderr,
        "⚠ Working Copy has an unrecorded edit to spec.md\n  this is the recorded spec rev 2\n"
    );
}

#[test]
fn show_refuses_an_unknown_artifact_listing_the_existing_ones() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let unknown = refused(surt(&sandbox, &["show", &name, "specs"]));
    let missing_revision = refused(surt(&sandbox, &["show", &name, "spec", "--rev", "3"]));
    let unknown_item = refused(surt(&sandbox, &["show", &name, "feedback", "--item", "A"]));

    assert_eq!(
        unknown,
        format!(
            "✖ {name} has no artifact \"specs\"\n  \
             artifacts: request, base, spec, spec-rejection-reason, manifest\n  \
             inspect: surt show {name}\n"
        )
    );
    assert_eq!(
        missing_revision,
        format!(
            "✖ spec has no rev 3; the latest is rev 2\n  \
             inspect: surt show {name} spec --rev 2\n"
        )
    );
    assert_eq!(
        unknown_item,
        format!(
            "✖ {name} has no item \"A\"\n  \
             it has no items yet\n  \
             inspect: surt status {name}\n"
        )
    );
}

fn reviewed_twice(name: &str) -> Sandbox {
    let sandbox = planned(
        name,
        &[
            tasks(1, &["A", "B"]),
            build("A", 1, 1, "login.txt", "login v1\n"),
            review("A", 1, "changes_requested"),
            build("A", 2, 1, "login.txt", "login v2\n"),
            review("A", 2, "approved"),
            build("B", 1, 1, "logout.txt", "logout\n"),
            review("B", 1, "approved"),
            ship(),
        ],
    );
    ok(surt(&sandbox, &["continue", name]));
    sandbox
}

fn summary(feedback: &str) -> String {
    let document: serde_json::Value = serde_json::from_str(feedback).expect("feedback JSON");
    document["summary"].as_str().expect("a summary").to_owned()
}

#[test]
fn iter_selects_the_revision_an_item_produced_in_that_iteration() {
    let name = subject_name();
    let sandbox = reviewed_twice(&name);

    let first = ok(surt(
        &sandbox,
        &["show", &name, "feedback", "--item", "A", "--iter", "1"],
    ));
    let second = ok(surt(
        &sandbox,
        &["show", &name, "feedback", "--item", "A", "--iter", "2"],
    ));
    let in_run_1 = ok(surt(
        &sandbox,
        &[
            "show", &name, "feedback", "--item", "A", "--iter", "1", "--run", "1",
        ],
    ));

    assert_eq!(
        summary(&first.stdout),
        "changes_requested after reading the code"
    );
    assert_eq!(summary(&second.stdout), "approved after reading the code");
    assert_eq!(in_run_1.stdout, first.stdout);
}

#[test]
fn an_iteration_that_produced_no_revision_is_refused_listing_those_that_did() {
    let name = subject_name();
    let sandbox = reviewed_twice(&name);

    let never = refused(surt(
        &sandbox,
        &["show", &name, "feedback", "--item", "A", "--iter", "3"],
    ));
    let unknown_artifact = refused(surt(
        &sandbox,
        &["show", &name, "feedbacks", "--item", "B", "--iter", "1"],
    ));

    assert_eq!(
        never,
        format!(
            "✖ item A produced no feedback in iter 3\n  \
             iters with feedback: 1, 2\n  \
             inspect: surt show {name} feedback --item A --iter 2\n"
        )
    );
    assert!(
        unknown_artifact.starts_with(&format!("✖ {name} has no artifact \"feedbacks\"\n")),
        "{unknown_artifact}"
    );
}

#[test]
fn iter_without_item_or_with_rev_is_invalid_usage() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let alone = surt(&sandbox, &["show", &name, "spec", "--iter", "1"]);
    let with_rev = surt(
        &sandbox,
        &[
            "show", &name, "spec", "--item", "A", "--iter", "1", "--rev", "1",
        ],
    );

    assert_eq!(alone.code, Some(USAGE_ERROR_EXIT_CODE), "{}", alone.stderr);
    assert_eq!(
        with_rev.code,
        Some(USAGE_ERROR_EXIT_CODE),
        "{}",
        with_rev.stderr
    );
    assert!(alone.stdout.is_empty());
}

#[test]
fn file_prints_one_file_of_a_revision_byte_for_byte() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let item = ok(surt(
        &sandbox,
        &["show", &name, "manifest", "--file", "tasks/B.md"],
    ));
    let in_run_1 = ok(surt(
        &sandbox,
        &[
            "show",
            &name,
            "manifest",
            "--file",
            "tasks/A.md",
            "--run",
            "1",
        ],
    ));
    let older = ok(surt(
        &sandbox,
        &["show", &name, "spec", "--rev", "1", "--file", "spec.md"],
    ));

    assert_eq!(item.stdout, "# Task B\n");
    assert_eq!(in_run_1.stdout, "# Task A\n");
    assert_eq!(older.stdout, SPEC_1);
}

#[test]
fn file_combines_with_item_and_iter() {
    let name = subject_name();
    let sandbox = reviewed_twice(&name);

    let first = ok(surt(
        &sandbox,
        &[
            "show",
            &name,
            "feedback",
            "--item",
            "A",
            "--iter",
            "1",
            "--file",
            "feedback.json",
        ],
    ));

    assert_eq!(
        summary(&first.stdout),
        "changes_requested after reading the code"
    );
}

#[test]
fn an_unknown_file_is_refused_listing_the_files_of_that_revision() {
    let name = subject_name();
    let sandbox = with_two_specs(&name);

    let unknown = refused(surt(
        &sandbox,
        &["show", &name, "manifest", "--file", "tasks/Z.md"],
    ));
    let older = refused(surt(
        &sandbox,
        &["show", &name, "spec", "--rev", "1", "--file", "a.md"],
    ));

    assert_eq!(
        older,
        "✖ spec rev 1 has no file \"a.md\"\n  files: spec.md\n"
    );
    assert_eq!(
        unknown,
        "✖ manifest rev 1 has no file \"tasks/Z.md\"\n  \
         files: manifest.json, tasks/A.md, tasks/B.md\n"
    );
}
