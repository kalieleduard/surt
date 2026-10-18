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

const TWIN: &str = "aaaaaaaa-0000-7000-8000-000000000000";
const OTHER_TWIN: &str = "aaaaaaaa-1111-7000-8000-000000000000";

fn crashed_build(sandbox: &Sandbox) -> String {
    sandbox
        .texts(
            "SELECT x.id FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.item_key = 'B' AND s.step_key = 'build' AND x.attempt = 1",
        )
        .remove(0)
}

fn copy_execution(sandbox: &Sandbox, from: &str, id: &str, attempt: u32) {
    sandbox.execute(&format!(
        "INSERT INTO execution (id, step_run_id, attempt, kind, agent_snapshot_sha256, \
         state, started_at, finished_at) SELECT '{id}', step_run_id, {attempt}, kind, \
         agent_snapshot_sha256, 'interrupted', started_at, finished_at \
         FROM execution WHERE id = '{from}'"
    ));
}

#[test]
fn inspect_shows_one_execution_by_its_full_id_after_cleanup_without_logs() {
    let name = feature_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    let id = crashed_build(&sandbox);
    fs::remove_dir_all(sandbox.dot_surt().join("logs")).expect("the logs are removed");
    let salvage = format!("refs/surt/salvage/{id}");
    sandbox.git(&sandbox.repo(), &["update-ref", &salvage, "HEAD"]);
    let mut before = hashes(&sandbox.dot_surt());
    before.retain(|path, _| !path.starts_with("surt.db-"));

    let shown = ok(surt(&sandbox, &["inspect", &id]));

    assert_unchanged(&sandbox.dot_surt(), before);
    assert!(
        !sandbox
            .dot_surt()
            .join("runtime/executions")
            .join(&id)
            .exists()
    );
    let lines: Vec<&str> = shown.lines().collect();
    assert_eq!(lines[0], format!("execution {id}"));
    assert_eq!(lines[1], format!("  feature   {name} · run 1 · task B"));
    assert!(
        lines[2].starts_with("  step      build iter 1 · attempt 1 · agent-"),
        "{shown}"
    );
    assert_eq!(lines[3], "  role      builder");
    assert!(
        lines[4].starts_with("  state     failed (retryable) · exit_code — "),
        "{shown}"
    );
    assert_eq!(
        lines[5],
        "  profile   claude-code · claude-sonnet-5-5 · effort high · dontAsk"
    );
    assert!(
        lines[6].starts_with("  session   ") && lines[6].contains(" · model "),
        "{shown}"
    );
    assert!(lines[7].starts_with("  process   exit "), "{shown}");
    assert!(
        lines.iter().any(|line| line.starts_with("  inputs    ")),
        "{shown}"
    );
    assert!(!shown.contains("turns") && !shown.contains('$'), "{shown}");
    let worktree = format!("{name}/B");
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("  worktree  ") && line.ends_with(&worktree)),
        "{shown}"
    );
    assert!(shown.ends_with(&format!(
        "  salvage   {salvage}\n  logs      surt logs {id}\n"
    )));
}

#[test]
fn inspect_finds_an_execution_by_an_eight_character_prefix() {
    let name = feature_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    copy_execution(&sandbox, &crashed_build(&sandbox), TWIN, 101);

    let shown = ok(surt(&sandbox, &["inspect", &TWIN[..8]]));

    assert!(shown.starts_with(&format!("execution {TWIN}\n")), "{shown}");
    assert!(shown.contains("  state     interrupted\n"), "{shown}");
    assert!(
        shown.ends_with(&format!("  logs      surt logs {TWIN}\n")),
        "{shown}"
    );
}

#[test]
fn inspect_refuses_a_too_short_or_ambiguous_prefix_and_lists_the_candidates() {
    let name = feature_name();
    let sandbox = delivered_after_a_rejected_review(&name);
    let crashed = crashed_build(&sandbox);
    copy_execution(&sandbox, &crashed, TWIN, 101);
    copy_execution(&sandbox, &crashed, OTHER_TWIN, 102);

    let short = surt(&sandbox, &["inspect", "aaaa"]);
    let ambiguous = surt(&sandbox, &["inspect", &TWIN[..8]]);
    let unknown = surt(&sandbox, &["inspect", "bbbbbbbb"]);

    for refused in [&short, &ambiguous, &unknown] {
        assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
        assert!(refused.stdout.is_empty(), "{}", refused.stdout);
    }
    let listed = |id: &str, attempt: u32| {
        format!(
            "    {id}  {name} · run 1 · task B · build iter 1 · attempt {attempt}  interrupted\n"
        )
    };
    assert_eq!(
        short.stderr,
        format!(
            "✖ \"aaaa\" is too short; an execution id prefix needs at least 8 characters\n{}{}  \
             use the full id, as surt prints it\n",
            listed(TWIN, 101),
            listed(OTHER_TWIN, 102)
        )
    );
    assert_eq!(
        ambiguous.stderr,
        format!(
            "✖ \"aaaaaaaa\" matches 2 executions:\n{}{}  use the full id, as surt prints it\n",
            listed(TWIN, 101),
            listed(OTHER_TWIN, 102)
        )
    );
    assert_eq!(
        unknown.stderr,
        "✖ no feature or execution matches \"bbbbbbbb\"\n  inspect: surt status\n"
    );
}
