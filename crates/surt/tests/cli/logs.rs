use std::fs;
use std::path::PathBuf;

use crate::delivering::subject_name;
use crate::scripts::{build, review, ship, spec, tasks};
use crate::status::with_a_blocked_item;
use crate::support::{REFUSED_EXIT_CODE, Run, Sandbox};

const SUCCESS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../surt-engine/tests/fixtures/claude-code/success.stdout.jsonl"
));
const SIGTERM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../surt-engine/tests/fixtures/claude-code/sigterm.stdout.jsonl"
));
const VERSION_CHECKS: &str = "[[checks]]\nname = \"first\"\nrun = [\"git\", \"--version\"]\n\n\
                              [[checks]]\nname = \"second\"\nrun = [\"git\", \"--version\"]\n";
const REVIEW: &str = concat!(
    r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Looks good"}]}}"#,
    "\n",
    r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"ok"}]}}"#,
    "\n"
);

fn ok(run: Run) -> String {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

fn surt(sandbox: &Sandbox, args: &[&str]) -> Run {
    sandbox.surt(&sandbox.repo(), args)
}

pub fn latest(sandbox: &Sandbox, item: &str, step: &str) -> String {
    sandbox
        .texts(&format!(
            "SELECT x.id FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.item_key IS {item} AND s.step_key = '{step}' ORDER BY x.rowid DESC LIMIT 1"
        ))
        .concat()
}

pub fn logs_of(sandbox: &Sandbox, name: &str, execution: &str) -> PathBuf {
    sandbox.dot_surt().join("logs").join(name).join(execution)
}

struct Logged {
    sandbox: Sandbox,
    name: String,
    plan: String,
    review: String,
    build: String,
}

fn logged() -> Logged {
    let name = subject_name();
    let sandbox = with_a_blocked_item(&name);
    let plan = latest(&sandbox, "NULL", "plan-tasks");
    let review = latest(&sandbox, "'A'", "review");
    let build = latest(&sandbox, "'B'", "build");
    for (execution, stream) in [(&plan, SUCCESS), (&review, REVIEW), (&build, SIGTERM)] {
        fs::write(
            logs_of(&sandbox, &name, execution).join("stream.jsonl"),
            stream,
        )
        .expect("a stream");
    }
    fs::write(
        logs_of(&sandbox, &name, &build).join("stderr.log"),
        "boom\n",
    )
    .expect("a stderr");
    Logged {
        sandbox,
        name,
        plan,
        review,
        build,
    }
}

#[test]
fn logs_of_a_subject_render_the_latest_execution_of_the_run_and_each_item_with_a_prefix() {
    let logged = logged();

    let shown = ok(surt(&logged.sandbox, &["logs", &logged.name]));

    assert_eq!(
        shown,
        "plan-tasks#1 │ Write delivery/note.md\n\
         plan-tasks#1 │ ✔ delivered\n\
         A review#1   │ \"Looks good\"\n\
         B build#3    │ Write delivery/a.md\n\
         B build#3    │ boom\n"
    );
    assert_eq!(
        ok(surt(&logged.sandbox, &["logs", &logged.name, "--run", "1"])),
        shown
    );
}

#[test]
fn logs_of_an_item_or_an_execution_have_no_prefix() {
    let logged = logged();
    let prefix = &logged.build[..logged.build.len() - 1];

    let item = ok(surt(&logged.sandbox, &["logs", &logged.name, "B"]));
    let execution = ok(surt(&logged.sandbox, &["logs", &logged.build]));
    let by_prefix = ok(surt(&logged.sandbox, &["logs", prefix]));
    let review = ok(surt(&logged.sandbox, &["logs", &logged.review]));

    assert_eq!(item, "Write delivery/a.md\nboom\n");
    assert_eq!(execution, item);
    assert_eq!(by_prefix, item);
    assert_eq!(review, "\"Looks good\"\n");
}

#[test]
fn raw_logs_print_the_original_stream_lines() {
    let logged = logged();

    let execution = ok(surt(&logged.sandbox, &["logs", &logged.plan, "--raw"]));
    let item = ok(surt(&logged.sandbox, &["logs", &logged.name, "B", "--raw"]));

    assert_eq!(execution, SUCCESS);
    assert_eq!(item, SIGTERM);
}

#[test]
fn following_a_run_without_a_live_driver_prints_what_exists_and_exits() {
    let logged = logged();
    let (name, plan) = (logged.name.as_str(), logged.plan.as_str());

    let subject = ok(surt(&logged.sandbox, &["logs", name, "-f"]));
    let item = ok(surt(&logged.sandbox, &["logs", name, "B", "--follow"]));
    let execution = ok(surt(&logged.sandbox, &["logs", plan, "-f", "--raw"]));

    assert_eq!(subject, ok(surt(&logged.sandbox, &["logs", name])));
    assert_eq!(item, "Write delivery/a.md\nboom\n");
    assert_eq!(execution, SUCCESS);
}

#[test]
fn a_missing_log_directory_says_so_and_points_at_inspect() {
    let logged = logged();
    let (name, build) = (&logged.name, &logged.build);
    fs::remove_dir_all(logs_of(&logged.sandbox, name, build)).expect("the log is removed");

    let refused = surt(&logged.sandbox, &["logs", build]);
    let subject = surt(&logged.sandbox, &["logs", name]);

    let missing = format!(
        "no log for execution {build} (.surt/logs/{name}/{build}/ is missing)\n  \
         inspect: surt inspect {build}\n"
    );
    assert_eq!(refused.code, Some(REFUSED_EXIT_CODE), "{}", refused.stderr);
    assert_eq!(refused.stdout, "");
    assert_eq!(refused.stderr, format!("✖ {missing}"));
    assert_eq!(subject.code, Some(0), "{}", subject.stderr);
    assert_eq!(subject.stderr, format!("⚠ {missing}"));
    assert_eq!(
        subject.stdout,
        "plan-tasks#1 │ Write delivery/note.md\n\
         plan-tasks#1 │ ✔ delivered\n\
         A review#1   │ \"Looks good\"\n"
    );
}

#[test]
fn a_check_shows_its_combined_output_under_one_header_per_check() {
    let name = subject_name();
    let sandbox = Sandbox::with_repository();
    sandbox.commit_something();
    sandbox.init();
    sandbox.write_config(VERSION_CHECKS);
    sandbox.script(&[
        spec(1, "# Spec 1\n"),
        tasks(1, &["A"]),
        build("A", 1, 1, "login.txt", "login\n"),
        review("A", 1, "approved"),
        ship(),
    ]);
    ok(sandbox.run_feature("add login", &[]));
    for command in ["approve", "continue", "approve", "continue"] {
        ok(surt(&sandbox, &[command, &name]));
    }
    let check = latest(&sandbox, "'A'", "check");
    let version = sandbox.git_output(&sandbox.repo(), &["--version"]);

    let shown = ok(surt(&sandbox, &["logs", &check]));

    assert_eq!(
        shown,
        format!("=== first: git --version\n{version}=== second: git --version\n{version}")
    );
}

#[test]
fn logs_of_an_unknown_item_or_run_refuse() {
    let logged = logged();

    let item = surt(&logged.sandbox, &["logs", &logged.name, "Z"]);
    let run = surt(&logged.sandbox, &["logs", &logged.name, "--run", "2"]);

    assert_eq!(item.code, Some(REFUSED_EXIT_CODE), "{}", item.stderr);
    assert!(item.stderr.contains("items: A, B"), "{}", item.stderr);
    assert_eq!(run.code, Some(REFUSED_EXIT_CODE), "{}", run.stderr);
    assert_eq!((item.stdout.as_str(), run.stdout.as_str()), ("", ""));
}
