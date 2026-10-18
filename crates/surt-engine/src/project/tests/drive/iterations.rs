use std::fs;

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, Recording, draft, label};
use crate::project::{Decided, DriveOutcome, Gate, RevisionOrigin};
use crate::runtime::Script;

const REASON: &str = "add a title";

async fn rejected_once() -> (Feature, Decided) {
    let feature = Feature::review().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the first draft reaches the gate");
    let rejected = feature
        .project
        .reject(&feature.run, Some(1), REASON, ACTOR)
        .await
        .expect("the rejection is recorded");
    (feature, rejected)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rejection_records_the_decision_on_the_reviewed_revision_without_driving() {
    let (feature, rejected) = rejected_once().await;

    let revision: Vec<(String,)> = feature
        .dir
        .rows("SELECT identity_sha256 FROM artifact_revision WHERE origin = 'execution'")
        .await;
    let approvals: Vec<(String, String, String)> = feature
        .dir
        .rows(
            "SELECT a.decision, a.identity_sha256, CAST(b.bytes AS TEXT) FROM approval a \
             JOIN revision_file f ON f.revision_id = a.reason_revision_id \
             JOIN blob b ON b.sha256 = f.blob_sha256",
        )
        .await;
    let executions: Vec<(i64,)> = feature.dir.rows("SELECT count(*) FROM execution").await;
    assert_eq!(
        rejected,
        Decided {
            gate: "approve".to_owned(),
            artifact: "draft".to_owned(),
            revision: 1,
            sha256: revision[0].0.clone(),
            origin: RevisionOrigin::Agent,
            actor: ACTOR.to_owned(),
        }
    );
    assert_eq!(
        approvals,
        [(
            "reject".to_owned(),
            revision[0].0.clone(),
            REASON.to_owned()
        )]
    );
    assert_eq!(executions, [(1,)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_next_iteration_reads_the_rejected_revision_the_reason_and_why_it_started() {
    let (feature, _) = rejected_once().await;
    let runtime = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate {
            gates: vec![Gate {
                step: label("approve", Some(2)),
                artifact: "draft".to_owned(),
            }]
        }
    );
    let requests = runtime.requests();
    let [request] = requests.as_slice() else {
        panic!("one Execution, got {requests:?}");
    };
    let inputs = request
        .delivery_dir
        .parent()
        .expect("the Execution directory")
        .join("inputs");
    let context = &request.context;
    assert!(
        context.contains(
            "## Why this Iteration started\nIteration 1 ended: write = done, approve = rejected\n"
        ),
        "{context}"
    );
    assert!(
        context.contains("- draft rev 1   (iteration 1, write)"),
        "{context}"
    );
    assert!(
        context.contains("- draft-rejection rev 1   (iteration 1, approve)"),
        "{context}"
    );
    assert!(context.contains("- request rev 1   (initial)"), "{context}");
    assert_eq!(
        fs::read_to_string(inputs.join("draft/draft.md")).expect("the rejected draft"),
        "# Draft 1\n"
    );
    assert_eq!(
        fs::read_to_string(inputs.join("draft-rejection/draft-rejection.txt")).expect("the reason"),
        REASON
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approval_of_the_latest_revision_lets_the_next_step_read_it() {
    let (feature, _) = rejected_once().await;
    feature
        .drive(Script::new().on("write", 2, 1, draft("# Draft 2\n")))
        .await
        .outcome
        .expect("the second draft reaches the gate");

    let approved = feature
        .project
        .approve(&feature.run, Some(2), ACTOR)
        .await
        .expect("the approval is recorded");
    let driven = feature
        .drive(Script::new().on("publish", 1, 1, super::fixtures::summary("ok\n")))
        .await;

    assert_eq!(approved.revision, 2);
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let read: Vec<(String, i64, bool)> = feature
        .dir
        .rows(
            "SELECT i.input_name, r.number, r.pinned_at IS NOT NULL FROM execution_input i \
             JOIN artifact_revision r ON r.id = i.revision_id JOIN execution e ON e.id = i.execution_id \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'publish'",
        )
        .await;
    assert_eq!(read, [("draft".to_owned(), 2, true)]);
    let decisions: Vec<(String, i64, bool)> = feature
        .dir
        .rows(
            "SELECT a.decision, r.number, a.identity_sha256 = r.identity_sha256 FROM approval a \
             JOIN artifact_revision r ON r.id = a.revision_id ORDER BY r.number",
        )
        .await;
    assert_eq!(
        decisions,
        [
            ("reject".to_owned(), 1, true),
            ("approve".to_owned(), 2, true)
        ]
    );
    assert_eq!(
        fs::read_to_string(
            feature
                .dir
                .path()
                .join(".surt/features")
                .join(SUBJECT)
                .join("draft.md")
        )
        .expect("the Working Copy"),
        "# Draft 2\n"
    );
}
