use std::fs;

use crate::scripts::{build, review, ship, tasks};
use crate::start::{feature_name, planned};
use crate::support::{INTERNAL_ERROR_EXIT_CODE, REFUSED_EXIT_CODE, Sandbox};

const ESCAPE: char = '\u{1b}';
const EVENT_MARKS: [char; 4] = ['●', '✔', '↻', ' '];
const NOT_A_DATABASE: &str = "this file is not an SQLite database, it only pretends to be one";
const CORRUPT_COPIES: usize = 128;

#[test]
fn an_unexpected_error_is_an_internal_error_that_suggests_surt_log() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    for sidecar in ["surt.db-wal", "surt.db-shm"] {
        let _ = fs::remove_file(sandbox.dot_surt().join(sidecar));
    }
    fs::write(
        sandbox.dot_surt().join("surt.db"),
        NOT_A_DATABASE.repeat(CORRUPT_COPIES),
    )
    .expect("a corrupt database");

    let run = sandbox.surt(&sandbox.repo(), &["status"]);

    assert_eq!(run.code, Some(INTERNAL_ERROR_EXIT_CODE), "{}", run.stderr);
    assert!(
        run.stderr.starts_with("✖ internal error: "),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .contains("\n  rerun with SURT_LOG=debug for details\n"),
        "{}",
        run.stderr
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
}

#[test]
fn surt_log_turns_on_diagnostics_on_stderr() {
    let sandbox = Sandbox::with_repository();

    let quiet = sandbox.surt(&sandbox.repo(), &["init"]);
    let verbose = sandbox.surt_with(&sandbox.repo(), &["init"], &[("SURT_LOG", "debug")]);

    assert!(!quiet.stderr.contains("DEBUG"), "{}", quiet.stderr);
    assert!(verbose.stderr.contains("DEBUG"), "{}", verbose.stderr);
    assert!(verbose.stderr.contains("running git"), "{}", verbose.stderr);
    assert!(verbose.stdout.is_empty(), "{}", verbose.stdout);
}

#[test]
fn no_color_wins_over_forced_colour() {
    let sandbox = Sandbox::with_repository();

    let coloured = sandbox.surt_with(&sandbox.repo(), &["status"], &[("CLICOLOR_FORCE", "1")]);
    let plain = sandbox.surt_with(
        &sandbox.repo(),
        &["status"],
        &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")],
    );

    assert_eq!(coloured.code, Some(REFUSED_EXIT_CODE));
    assert!(coloured.stderr.contains(ESCAPE), "{:?}", coloured.stderr);
    assert_eq!(plain.code, Some(REFUSED_EXIT_CODE));
    assert!(!plain.stderr.contains(ESCAPE), "{:?}", plain.stderr);
    assert!(
        plain.stderr.starts_with("✖ not a surt project"),
        "{}",
        plain.stderr
    );
}

#[test]
fn a_driving_command_without_a_tty_prints_only_event_lines() {
    let name = feature_name();
    let sandbox = planned(
        &name,
        &[
            tasks(1, &["A", "B"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            build("B", 1, 1, "logout.txt", "logout\n"),
            review("B", 1, "approved"),
            ship(),
        ],
    );

    let run = sandbox.surt(&sandbox.repo(), &["start", &name]);

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stderr.contains("● task B build started\n"),
        "{}",
        run.stderr
    );
    assert!(!run.stderr.contains(['\r', ESCAPE]), "{:?}", run.stderr);
    assert!(!run.stderr.contains("running ("), "{}", run.stderr);
    assert!(!run.stderr.contains("still running"), "{}", run.stderr);
    for line in run.stderr.lines() {
        assert!(line.starts_with(EVENT_MARKS), "{line:?}\n{}", run.stderr);
    }
}
