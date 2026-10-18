use std::fs;

use crate::scripts::{build, crash_in, review, ship, spec, tasks};
use crate::start::feature_name;
use crate::status::{assert_unchanged, hashes};
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

fn delivered_after_a_rejected_review(name: &str) -> Sandbox {
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.script(&[
        spec(1, "# Spec 1\n"),
        spec(2, "# Spec 2 with OAuth\n"),
        tasks(1, &["A", "B"]),
        build("A", 1, 1, "login.txt", "login v1\n"),
        review("A", 1, "changes_requested"),
        build("A", 2, 1, "login.txt", "login v2\n"),
        review("A", 2, "approved"),
        crash_in("B/build", 1, 1),
        build("B", 1, 2, "logout.txt", "logout\n"),
        review("B", 1, "approved"),
        ship(),
    ]);
    ok(sandbox.feature(&["add login"]));
    ok(surt(
        &sandbox,
        &["reject", name, "-m", "needs OAuth\nand more"],
    ));
    ok(sandbox.feature(&[name]));
    for command in ["approve", "plan", "approve", "start"] {
        ok(surt(&sandbox, &[command, name]));
    }
    sandbox
}

fn details<'a>(timeline: &'a str, kind: &str) -> Vec<&'a str> {
    timeline
        .lines()
        .filter_map(|line| {
            let (time, rest) = line.split_once("  ")?;
            assert!(!time.trim().is_empty(), "{line}");
            let rest = rest.trim_start();
            let (found, details) = rest.split_once(' ').unwrap_or((rest, ""));
            (found == kind).then_some(details.trim_start())
        })
        .collect()
}

#[test]
fn inspect_explains_a_rejected_spec_and_a_new_builder_iteration_after_cleanup_without_logs() {
    let name = feature_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    fs::remove_dir_all(sandbox.dot_surt().join("logs")).expect("the logs are removed");
    let before = hashes(&sandbox.dot_surt());

    let run = ok(surt(&sandbox, &["inspect", &name]));
    let task = ok(surt(&sandbox, &["inspect", &name, "A", "--run", "1"]));
    let crashed = ok(surt(&sandbox, &["inspect", &name, "B"]));

    assert_unchanged(&sandbox.dot_surt(), before);
    assert_eq!(details(&run, "run.created")[1..], ["task A", "task B"]);
    assert_eq!(
        details(&run, "approval.recorded")[0],
        "approve-spec iter 1 · spec rev 1 rejected — \"needs OAuth\" · by tester@example.com"
    );
    assert_eq!(
        details(&run, "iteration.started")[1],
        "spec-loop · iter 2 until approve-spec not met: rejected"
    );
    assert_eq!(details(&run, "cleanup.finished"), [""]);
    assert!(
        details(&run, "step.running")
            .iter()
            .all(|step| !step.starts_with("build"))
    );
    assert_eq!(
        details(&task, "step.done")[2..4],
        ["check iter 1 · passed", "review iter 1 · changes_requested"]
    );
    assert_eq!(
        details(&task, "iteration.started"),
        ["cycle · iter 1", "cycle · iter 2 until integrate not met"]
    );
    let builds: Vec<_> = details(&task, "execution.started")
        .into_iter()
        .filter(|started| started.starts_with("build"))
        .collect();
    assert_eq!(builds.len(), 2, "{task}");
    assert!(
        builds[1].starts_with("build iter 2 · attempt 1 · agent-"),
        "{task}"
    );
    let failed = details(&crashed, "execution.failed");
    assert_eq!(failed.len(), 1, "{crashed}");
    assert!(
        failed[0].starts_with("build iter 1 · attempt 1 · exit_code — "),
        "{crashed}"
    );
}

#[test]
fn inspect_of_an_unknown_task_or_run_refuses_and_lists_what_exists() {
    let name = feature_name();
    let sandbox = delivered_after_a_rejected_review(&name);

    let task = surt(&sandbox, &["inspect", &name, "Z"]);
    let run = surt(&sandbox, &["inspect", &name, "--run", "2"]);

    assert_eq!(task.code, Some(REFUSED_EXIT_CODE), "{}", task.stderr);
    assert!(task.stdout.is_empty(), "{}", task.stdout);
    assert_eq!(
        task.stderr,
        format!("✖ {name} has no task \"Z\"\n  tasks: A, B\n  inspect: surt status {name}\n")
    );
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
}
