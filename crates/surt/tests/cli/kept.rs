use serde_json::Value;

use crate::delivering::{planned_with, subject_name};
use crate::scripts::{build, review, ship, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const KEEP: &str = "[cleanup]\non_success = \"keep\"\n";

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn shown(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

fn kept(name: &str) -> (Sandbox, String) {
    let sandbox = planned_with(
        name,
        &[
            tasks(1, &["A"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            ship(),
        ],
        Some(KEEP),
    );
    let started = surt(&sandbox, &["continue", name]);
    assert!(started.stdout.is_empty(), "{}", started.stdout);
    (sandbox, ok(started))
}

#[test]
fn a_kept_run_ends_with_the_kept_line_instead_of_a_cleanup() {
    let name = subject_name();

    let (sandbox, started) = kept(&name);

    assert!(
        started.ends_with(&format!(
            "✔ run succeeded\n· cleanup kept (on_success = \"keep\") — remove with: surt delete {name}\n"
        )),
        "{started}"
    );
    assert!(!started.contains("↻ cleanup"), "{started}");
    assert_eq!(
        sandbox
            .texts("SELECT CAST(count(*) AS TEXT) FROM workflow_run WHERE cleaned_at IS NOT NULL")
            .concat(),
        "0"
    );
}

#[test]
fn status_shows_a_kept_run_as_kept_and_never_as_cleanup_pending() {
    let name = subject_name();
    let (sandbox, _) = kept(&name);

    let all = shown(surt(&sandbox, &["status", "--all"]));
    let detail = shown(surt(&sandbox, &["status", &name]));
    let json = surt(&sandbox, &["status", "--all", "--json"]);

    assert!(
        all.contains(&format!(
            "{name}  1    SUCCEEDED  —       1/1 merged · kept\n"
        )),
        "{all}"
    );
    assert!(!all.contains("pending"), "{all}");
    assert!(
        detail.ends_with(&format!(
            "\n  cleanup  kept (on_success = \"keep\") — remove with: surt delete {name}\n"
        )),
        "{detail}"
    );
    assert_eq!(json.code, Some(0), "{}", json.stderr);
    let document: Value = serde_json::from_str(&json.stdout).expect("one JSON document");
    assert_eq!(document["subjects"][0]["cleanup"]["state"], "kept");
}

#[test]
fn a_phase_command_on_a_kept_run_removes_nothing() {
    let name = subject_name();
    let (sandbox, _) = kept(&name);
    let worktrees = sandbox.git_output(&sandbox.repo(), &["worktree", "list"]);

    let again = surt(&sandbox, &["continue", &name]);

    assert_ne!(again.code, Some(0), "{}", again.stderr);
    assert!(!again.stderr.contains("cleanup"), "{}", again.stderr);
    assert_eq!(
        sandbox.git_output(&sandbox.repo(), &["worktree", "list"]),
        worktrees
    );
}

#[test]
fn a_misspelled_cleanup_policy_refuses_every_command_with_its_line() {
    let sandbox = Sandbox::with_repository();
    sandbox.init();
    sandbox.write_config("max_attempts = 3\n[cleanup]\non_success = \"keeep\"\n");

    for args in [
        &["status"][..],
        &["continue", "anything"],
        &["run", "feature", "--input", "request=add login"],
    ] {
        let run = surt(&sandbox, args);

        assert_eq!(
            run.code,
            Some(REFUSED_EXIT_CODE),
            "{args:?}: {}",
            run.stderr
        );
        assert!(
            run.stderr.starts_with("✖ invalid configuration in "),
            "{args:?}: {}",
            run.stderr
        );
        assert!(run.stderr.contains("config.toml:3"), "{}", run.stderr);
        assert!(run.stderr.contains("keeep"), "{}", run.stderr);
    }
}
