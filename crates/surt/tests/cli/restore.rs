use std::fs;
use std::path::PathBuf;

use crate::scripts::{build, review, ship, tasks};
use crate::start::{feature_name, planned};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const EDITED: &str = "# Spec 1, edited after planning\n";

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn spec_file(sandbox: &Sandbox, name: &str) -> PathBuf {
    sandbox
        .dot_surt()
        .join("features")
        .join(name)
        .join("spec.md")
}

fn human_edits(sandbox: &Sandbox) -> String {
    sandbox
        .texts(
            "SELECT CAST(count(*) AS TEXT) FROM artifact_revision \
             WHERE origin = 'human' AND human_action = 'edit'",
        )
        .concat()
}

fn pinned_and_edited(name: &str) -> Sandbox {
    let sandbox = planned(
        name,
        &[
            tasks(1, &["A"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
    );
    fs::write(spec_file(&sandbox, name), EDITED).expect("the edit is written");
    sandbox
}

#[test]
fn a_pinned_edit_refusal_is_undone_with_restore_yes_and_the_run_continues() {
    let name = feature_name();
    let sandbox = pinned_and_edited(&name);

    let refused = surt(&sandbox, &["start", &name]);
    let restored = surt(&sandbox, &["restore", &name, "spec", "--yes"]);
    let started = surt(&sandbox, &["start", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert!(
        refused.stderr.ends_with(&format!(
            "✖ spec.md changed, but spec rev 1 is pinned (a later step already read it)\n  \
             the edit was not recorded\n  undo: surt restore {name} spec\n"
        )),
        "{}",
        refused.stderr
    );
    assert_eq!(restored.code, Some(0), "{}", restored.stderr);
    assert_eq!(
        restored.stderr,
        "✔ restored spec rev 1; your edit was discarded\n"
    );
    assert_eq!(started.code, Some(0), "{}", started.stderr);
    assert!(
        started.stderr.contains("✔ run succeeded\n"),
        "{}",
        started.stderr
    );
    assert_eq!(human_edits(&sandbox), "0");
    assert_eq!(
        sandbox.texts("SELECT data FROM event WHERE kind = 'working_copy.restored'"),
        [r#"{"artifact":"spec","changed":["spec.md"],"missing":[],"new":[],"revision":1}"#]
    );
}

#[test]
fn restore_without_a_terminal_and_without_yes_refuses_and_keeps_the_edit() {
    let name = feature_name();
    let sandbox = pinned_and_edited(&name);

    let refused = surt(&sandbox, &["restore", &name, "spec"]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ restore asks for a confirmation and there is no terminal to ask\n  \
             fix: surt restore {name} spec --yes\n"
        )
    );
    assert_eq!(
        fs::read_to_string(spec_file(&sandbox, &name)).expect("the spec"),
        EDITED
    );
}

#[test]
fn restore_with_nothing_different_says_so_and_exits_0() {
    let name = feature_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);

    let restored = surt(&sandbox, &["restore", &name, "spec"]);

    assert_eq!(restored.code, Some(0), "{}", restored.stderr);
    assert_eq!(
        restored.stderr,
        "✔ spec already matches rev 1; nothing to restore\n"
    );
    assert!(restored.stdout.is_empty(), "{}", restored.stdout);
}

#[test]
fn restore_of_an_artifact_without_a_working_copy_lists_the_restorable_ones() {
    let name = feature_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);

    let refused = surt(&sandbox, &["restore", &name, "request", "--yes"]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ request has no working copy to restore\n  restorable: manifest, spec\n  \
             inspect: surt status {name}\n"
        )
    );
}
