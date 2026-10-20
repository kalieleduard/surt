use std::collections::BTreeSet;
use std::fs;

use uuid::Uuid;

use super::super::fixtures::SUBJECT;
use super::super::refusal;
use super::fixtures::{ACTOR, Feature, Recording, draft, label};
use crate::project::{DriveOutcome, Gate, GateKind, Progress, Refusal};
use crate::runtime::{Behavior, Script};

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_delivery_stops_at_the_approval_gate_that_reviews_it() {
    let feature = Feature::review().await;

    let driven = feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate {
            gates: vec![Gate {
                step: label("approve", Some(1)),
                kind: GateKind::Approval,
                artifact: "draft".to_owned(),
            }]
        }
    );
    let revisions: Vec<(String, i64, String, Vec<u8>)> = feature
        .dir
        .rows(
            "SELECT a.name, r.number, r.origin, b.bytes FROM artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id JOIN revision_file f ON f.revision_id = r.id \
             JOIN blob b ON b.sha256 = f.blob_sha256 WHERE r.origin = 'execution'",
        )
        .await;
    assert_eq!(
        revisions,
        [(
            "draft".to_owned(),
            1,
            "execution".to_owned(),
            b"# Draft 1\n".to_vec()
        )]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn each_execution_reports_its_progress_and_logs_under_the_subject() {
    let feature = Feature::review().await;

    let driven = feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await;

    assert_eq!(
        driven.progress,
        [
            Progress::StepStarted {
                step: label("write", Some(1)),
                attempt: 1,
            },
            Progress::RevisionPublished {
                artifact: "draft".to_owned(),
                revision: 1,
            },
            Progress::StepFinished {
                step: label("write", Some(1)),
                outcome: "done".to_owned(),
            },
        ]
    );
    let executions: Vec<(String,)> = feature.dir.rows("SELECT id FROM execution").await;
    let logs = feature
        .dir
        .path()
        .join(".surt/logs")
        .join(SUBJECT)
        .join(&executions[0].0);
    let stream = fs::read_to_string(logs.join("stream.jsonl")).expect("the stream log");
    assert!(!stream.is_empty());
    assert!(logs.join("stderr.log").is_file());
}

#[tokio::test(flavor = "multi_thread")]
async fn deciding_refuses_when_no_gate_waits_or_the_revision_is_not_the_latest() {
    let feature = Feature::review().await;
    let early = feature.project.approve(&feature.run, None, ACTOR).await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches the gate");

    let stale = feature
        .project
        .reject(&feature.run, Some(2), "no", ACTOR)
        .await;

    assert_eq!(
        refusal(early),
        Refusal::NoGateWaiting {
            subject: SUBJECT.to_owned()
        }
    );
    assert_eq!(
        refusal(stale),
        Refusal::StaleRevision {
            subject: SUBJECT.to_owned(),
            artifact: "draft".to_owned(),
            expected: 2,
            latest: 1,
        }
    );
    let decisions: Vec<(i64,)> = feature.dir.rows("SELECT count(*) FROM approval").await;
    assert_eq!(decisions, [(0,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_that_keeps_failing_blocks_after_the_attempt_limit() {
    let feature = Feature::review().await;
    let crash = || Behavior::Crash { exit_code: 1 };
    let runtime = Recording::new(
        Script::new()
            .on("write", 1, 1, crash())
            .on("write", 1, 2, crash())
            .on("write", 1, 3, crash()),
    );

    let driven = feature.drive_with(&runtime).await;

    let sessions: BTreeSet<Uuid> = runtime
        .requests()
        .iter()
        .map(|request| request.session_id)
        .collect();
    assert_eq!(sessions.len(), 3);

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked {
            steps: vec![label("write", Some(1))]
        }
    );
    let failures = driven
        .progress
        .iter()
        .filter(|progress| {
            matches!(progress, Progress::AttemptFailed { code, .. } if code == "exit_code")
        })
        .count();
    assert_eq!(failures, 3);
}
