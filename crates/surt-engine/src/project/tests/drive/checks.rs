use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::super::fixtures::SUBJECT;
use super::evidence::{Committing, change, committed, task_branch};
use super::fixtures::Feature;
use super::planned::review;
use super::repository::commit_file;
use crate::project::{CHECK_REPORT_SCHEMA, Check, Config, DriveOutcome};
use crate::runtime::{RuntimeRequest, Script};

const CHECKED_WORKFLOW: &str = r#"
schema = 1
name = "task"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"

[steps.check]
kind = "operation"
run = "checks.run"
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["check"]
when = { step = "check", outcome = "passed" }

[steps.cycle]
kind = "loop"
body = ["build", "check", "review"]
until = { step = "review", outcome = "approved" }
needs = ["setup"]
"#;

fn check(name: &str, run: &[&str]) -> Check {
    Check {
        name: name.to_owned(),
        run: run.iter().map(|argument| (*argument).to_owned()).collect(),
        timeout: None,
    }
}

fn with_checks(checks: Vec<Check>) -> Config {
    Config {
        checks,
        ..Config::default()
    }
}

fn report_read_by(requests: &[RuntimeRequest], step: &str, iteration: u32) -> Option<Value> {
    let request = requests
        .iter()
        .find(|request| request.step.step == step && request.step.iteration == iteration)?;
    let line = request
        .context
        .lines()
        .find(|line| line.starts_with("- check-report rev "))?;
    let root = PathBuf::from(line.rsplit("   ").next()?);
    let bytes = fs::read(root.join("check-report.json")).expect("the report file");
    let report: Value = serde_json::from_slice(&bytes).expect("the report is JSON");
    let (_, schema) = CHECK_REPORT_SCHEMA;
    let schema: Value = serde_json::from_str(schema).expect("the schema is JSON");
    let validator = jsonschema::draft7::new(&schema).expect("the schema compiles");
    assert!(validator.is_valid(&report), "{report}");
    Some(report)
}

fn without_durations(mut report: Value) -> Value {
    for check in report["checks"].as_array_mut().expect("a list of checks") {
        let duration = check
            .as_object_mut()
            .expect("a check")
            .remove("duration_ms")
            .expect("a duration");
        assert!(duration.is_u64(), "{duration}");
    }
    report
}

async fn check_outcomes(feature: &Feature) -> Vec<(u32, String)> {
    feature
        .dir
        .rows(
            "SELECT iteration, coalesce(outcome, state) FROM step_run \
             WHERE step_key IN ('check', 'review') ORDER BY iteration, step_key",
        )
        .await
}

fn output_log(feature: &Feature, execution: &str) -> String {
    let path = feature
        .root()
        .join(".surt/logs")
        .join(SUBJECT)
        .join(execution)
        .join("output.log");
    fs::read_to_string(path).expect("the output log")
}

async fn check_executions(feature: &Feature) -> Vec<(String,)> {
    feature
        .dir
        .rows(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'check' ORDER BY s.iteration, e.attempt",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn passing_checks_run_as_argv_in_the_task_worktree_on_the_builders_commit() {
    let config = with_checks(vec![
        check("head", &["git", "rev-parse", "HEAD"]),
        check(
            "literal",
            &["git", "log", "-1", "--format=%s $HOME && exit 1 | cat"],
        ),
    ]);
    let feature = committed(CHECKED_WORKFLOW, config).await;
    let runtime =
        Committing::new(Script::new().on("A/review", 1, 1, review("approved"))).on(1, 1, change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let sha = feature.sha(&task_branch()).expect("the task branch");
    let report = report_read_by(&runtime.requests(), "A/review", 1).expect("the review reads it");
    assert_eq!(
        without_durations(report),
        json!({
            "commit": sha,
            "checks": [
                { "name": "head", "status": "passed", "exit_code": 0, "output_tail": format!("{sha}\n") },
                { "name": "literal", "status": "passed", "exit_code": 0,
                  "output_tail": "feature.txt $HOME && exit 1 | cat\n" },
            ]
        })
    );
    assert_eq!(
        check_outcomes(&feature).await,
        [(1, "passed".to_owned()), (1, "approved".to_owned())]
    );
    let executions = check_executions(&feature).await;
    let log = output_log(&feature, &executions[0].0);
    assert!(log.contains(&format!("{sha}\n")), "{log}");
    assert!(log.contains("feature.txt $HOME && exit 1 | cat\n"), "{log}");
}

#[tokio::test(flavor = "multi_thread")]
async fn without_checks_the_check_passes_with_an_empty_report() {
    let feature = committed(CHECKED_WORKFLOW, Config::default()).await;
    let runtime =
        Committing::new(Script::new().on("A/review", 1, 1, review("approved"))).on(1, 1, change);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let sha = feature.sha(&task_branch()).expect("the task branch");
    assert_eq!(
        report_read_by(&runtime.requests(), "A/review", 1),
        Some(json!({ "commit": sha, "checks": [] }))
    );
}

fn add_missing_file(worktree: &Path) -> String {
    commit_file(worktree, "missing.txt", "found\n")
}

fn statuses(report: &Value) -> Vec<(&str, &str, Value)> {
    report["checks"]
        .as_array()
        .expect("the checks")
        .iter()
        .map(|check| {
            (
                check["name"].as_str().expect("a name"),
                check["status"].as_str().expect("a status"),
                check["exit_code"].clone(),
            )
        })
        .collect()
}

fn tail(report: &Value, index: usize) -> &str {
    report["checks"][index]["output_tail"]
        .as_str()
        .expect("an output tail")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_check_skips_review_and_the_next_builder_reads_the_check_report() {
    let config = with_checks(vec![
        check("present", &["git", "cat-file", "-e", "HEAD:missing.txt"]),
        check("head", &["git", "rev-parse", "HEAD"]),
    ]);
    let feature = committed(CHECKED_WORKFLOW, config).await;
    let runtime = Committing::new(Script::new().on("A/review", 2, 1, review("approved")))
        .on(1, 1, change)
        .on(2, 1, add_missing_file);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(
        check_outcomes(&feature).await,
        [
            (1, "failed".to_owned()),
            (1, "skipped".to_owned()),
            (2, "passed".to_owned()),
            (2, "approved".to_owned())
        ]
    );
    let requests = runtime.requests();
    let reviewed: Vec<u32> = requests
        .iter()
        .filter(|request| request.step.step == "A/review")
        .map(|request| request.step.iteration)
        .collect();
    assert_eq!(reviewed, [2]);
    let tip = feature.sha(&task_branch()).expect("the task branch");
    let first = feature.git(&["rev-parse", &format!("{tip}^")]);
    let report = report_read_by(&requests, "A/build", 2).expect("the builder reads it");
    assert_eq!(report["commit"], json!(first));
    assert_eq!(
        statuses(&report),
        [
            ("present", "failed", json!(128)),
            ("head", "passed", json!(0))
        ]
    );
    assert!(tail(&report, 0).contains("missing.txt"), "{report}");
    assert_eq!(tail(&report, 1), format!("{first}\n"));
    assert_eq!(report_read_by(&requests, "A/build", 1), None);
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_check_that_cannot_start_or_outlives_its_timeout_fails_and_the_rest_still_run() {
    let slow = Check {
        timeout: Some("200ms".to_owned()),
        ..check("slow", &["sleep", "30"])
    };
    let config = with_checks(vec![
        check("absent", &["surt-test-no-such-program"]),
        slow,
        check("head", &["git", "rev-parse", "HEAD"]),
    ]);
    let feature = committed(CHECKED_WORKFLOW, config).await;
    let runtime = Committing::new(Script::new()).on(1, 1, change);

    let driven = feature.drive_with(&runtime).await;

    assert!(
        matches!(driven.outcome, Ok(DriveOutcome::Blocked { .. })),
        "{:?}",
        driven.outcome
    );
    let report = report_read_by(&runtime.requests(), "A/build", 2).expect("the builder reads it");
    assert_eq!(
        statuses(&report),
        [
            ("absent", "not_started", Value::Null),
            ("slow", "timed_out", Value::Null),
            ("head", "passed", json!(0)),
        ]
    );
    assert!(
        tail(&report, 0).contains("surt-test-no-such-program"),
        "{report}"
    );
    let waited = report["checks"][1]["duration_ms"]
        .as_u64()
        .expect("a duration");
    assert!((200..10_000).contains(&waited), "{waited}");
}
