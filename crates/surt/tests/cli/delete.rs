use std::fs;

use crate::delivering::{planned, subject_name};
use crate::scripts::{build, crash_in, review, ship, spec, tasks};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const BLOCKED_EXIT_CODE: i32 = 4;

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stderr
}

fn surt_refs(sandbox: &Sandbox) -> String {
    sandbox.git_output(
        &sandbox.repo(),
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/surt/",
            "refs/surt/",
        ],
    )
}

fn subjects(sandbox: &Sandbox) -> Vec<String> {
    sandbox.texts("SELECT name FROM subject")
}

fn cancelled_with_a_blocked_item(name: &str) -> Sandbox {
    let sandbox = planned(
        name,
        &[
            tasks(1, &["A", "B"]),
            build("A", 1, 1, "login.txt", "login\n"),
            review("A", 1, "approved"),
            crash_in("B/build", 1, 1),
            crash_in("B/build", 1, 2),
            crash_in("B/build", 1, 3),
        ],
    );
    let blocked = surt(&sandbox, &["continue", name]);
    assert_eq!(blocked.code, Some(BLOCKED_EXIT_CODE), "{}", blocked.stderr);
    ok(surt(&sandbox, &["cancel", "--yes", name]));
    sandbox
}

fn delivered_as_run_two(sandbox: &Sandbox, name: &str) {
    sandbox.script(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    ok(surt(sandbox, &["continue", name, "--new-run"]));
    ok(surt(sandbox, &["approve", name]));
    ok(surt(sandbox, &["continue", name]));
    ok(surt(sandbox, &["approve", name]));
    ok(surt(sandbox, &["continue", name]));
}

#[test]
fn delete_after_a_cancelled_and_a_succeeded_run_keeps_only_the_delivered_branch() {
    let name = subject_name();
    let sandbox = cancelled_with_a_blocked_item(&name);
    delivered_as_run_two(&sandbox, &name);
    let refs = surt_refs(&sandbox);
    assert!(
        refs.contains(&format!("refs/heads/surt/{name}\n")),
        "{refs}"
    );
    assert!(
        refs.contains(&format!("refs/heads/surt/item/{name}/B\n")),
        "{refs}"
    );

    let deleted = ok(surt(&sandbox, &["delete", &name, "--yes"]));

    assert_eq!(deleted, format!("✔ {name} deleted\n"));
    assert_eq!(surt_refs(&sandbox), format!("refs/heads/surt/{name}.2\n"));
    assert_eq!(subjects(&sandbox), Vec::<String>::new());
    assert_eq!(
        sandbox.texts("SELECT CAST(count(*) AS TEXT) FROM blob"),
        ["0"]
    );
    let dot_surt = sandbox.dot_surt();
    assert!(!dot_surt.join("logs").join(&name).exists());
    assert!(!dot_surt.join("subjects").join(&name).exists());
    let mut locks: Vec<String> = fs::read_dir(dot_surt.join("runtime/locks"))
        .expect("the locks directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|file| file != "db.lock" && file != "git.lock")
        .collect();
    locks.sort();
    assert_eq!(locks, Vec::<String>::new());
    let worktrees = sandbox.git_output(&sandbox.repo(), &["worktree", "list"]);
    assert_eq!(worktrees.lines().count(), 1, "{worktrees}");
    let status = surt(&sandbox, &["status", "--all"]);
    assert!(!status.stdout.contains(&name), "{}", status.stdout);
}

#[test]
fn delete_without_a_terminal_and_without_yes_refuses_and_changes_nothing() {
    let name = subject_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);
    ok(surt(&sandbox, &["cancel", "--yes", &name]));

    let refused = surt(&sandbox, &["delete", &name]);

    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(
        refused.stderr,
        format!(
            "✖ delete asks for a confirmation and there is no terminal to ask\n  \
             fix: surt delete {name} --yes\n"
        )
    );
    assert_eq!(subjects(&sandbox), [name]);
}

#[test]
fn delete_of_a_subject_with_an_active_run_points_at_cancel_and_changes_nothing() {
    let name = subject_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);

    for args in [vec!["delete", &name], vec!["delete", &name, "--yes"]] {
        let refused = surt(&sandbox, &args);

        assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
        assert_eq!(
            refused.stderr,
            format!(
                "✖ {name} has an active run; delete never stops agents\n  fix: surt cancel {name}\n"
            )
        );
    }
    assert_eq!(subjects(&sandbox), [name]);
}

#[test]
fn a_delete_that_cannot_remove_a_directory_exits_1_and_the_rerun_finishes() {
    let name = subject_name();
    let sandbox = planned(&name, &[tasks(1, &["A"])]);
    ok(surt(&sandbox, &["cancel", "--yes", &name]));
    let logs = sandbox.dot_surt().join("logs").join(&name);
    fs::remove_dir_all(&logs).expect("the logs are replaced");
    fs::write(&logs, "not a directory\n").expect("a file in place of the logs");

    let failed = surt(&sandbox, &["delete", &name, "--yes"]);

    assert_eq!(failed.code, Some(1), "{}", failed.stderr);
    assert!(
        failed.stderr.starts_with(&format!(
            "✖ delete incomplete — could not remove .surt/logs/{name} ("
        )),
        "{}",
        failed.stderr
    );
    assert!(
        failed
            .stderr
            .ends_with(&format!("\n  retry: surt delete {name}\n")),
        "{}",
        failed.stderr
    );
    assert_eq!(subjects(&sandbox), std::slice::from_ref(&name));
    fs::remove_file(&logs).expect("the blocker is gone");

    ok(surt(&sandbox, &["delete", &name, "--yes"]));

    assert_eq!(subjects(&sandbox), Vec::<String>::new());
}
