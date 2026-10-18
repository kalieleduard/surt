use serde_json::json;

use super::fixtures::{ACTOR, Feature, draft};
use crate::project::DriveOutcome;
use crate::runtime::{Behavior, Script};

fn written() -> Script {
    Script::new().on("write", 1, 1, draft("# Draft\n"))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driver_records_its_pid_when_it_starts_and_when_it_stops() {
    let feature = Feature::review().await;

    let outcome = feature.drive(written()).await.outcome;

    assert!(
        matches!(outcome, Ok(DriveOutcome::AtGate { .. })),
        "{outcome:?}"
    );
    let pid = json!({ "pid": std::process::id() });
    assert_eq!(
        feature.dir.event_data("driver.started").await,
        std::slice::from_ref(&pid)
    );
    assert_eq!(feature.dir.event_data("driver.stopped").await, [pid]);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_execution_records_its_attempt_slot_and_runtime_and_what_it_published() {
    let feature = Feature::review().await;

    feature.drive(written()).await;

    assert_eq!(
        feature.dir.event_data("execution.started").await,
        [json!({ "attempt": 1, "slot": 0, "runtime": "fake" })]
    );
    assert_eq!(
        feature.dir.event_data("revision.published").await,
        [json!({ "artifact": "draft", "revision": 1, "origin": "execution" })]
    );
    assert_eq!(
        feature.dir.event_data("execution.succeeded").await,
        [json!({ "outcome": "done" })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rejection_records_its_decision_and_starts_the_next_iteration_with_its_cause() {
    let feature = Feature::review().await;
    let script = written().on("write", 2, 1, draft("# Draft 2\n"));
    feature.drive(script.clone()).await;

    feature
        .project
        .reject(
            &feature.run,
            Some(1),
            "Too short.\nAdd the error cases.",
            ACTOR,
        )
        .await
        .expect("the draft is rejected");
    feature.drive(script).await;

    assert_eq!(
        feature.dir.event_data("approval.recorded").await,
        [json!({
            "decision": "reject",
            "artifact": "draft",
            "revision": 1,
            "actor": ACTOR,
            "reason": "Too short."
        })]
    );
    assert_eq!(
        feature.dir.event_data("iteration.started").await,
        [
            json!({ "iteration": 1, "cause": "loop_started" }),
            json!({
                "iteration": 2,
                "cause": "until_not_met",
                "step": "approve",
                "outcome": "rejected"
            }),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn failures_block_with_their_reason_and_a_rerun_unblocks_with_the_new_budget_base() {
    let feature = Feature::review().await;
    let crashes = (1..=3).fold(Script::new(), |script, attempt| {
        script.on("write", 1, attempt, Behavior::Crash { exit_code: 1 })
    });
    feature.drive(crashes).await;

    feature
        .drive(Script::new().on("write", 1, 4, draft("# Draft\n")))
        .await;

    let crashed = json!({ "code": "exit_code", "message": "the agent exited with code 1" });
    assert_eq!(
        feature.dir.event_data("execution.failed").await,
        [crashed.clone(), crashed.clone(), crashed]
    );
    assert_eq!(
        feature.dir.event_data("step.blocked").await,
        [json!({ "reason": "attempts_exhausted" })]
    );
    assert_eq!(
        feature.dir.event_data("step.unblocked").await,
        [json!({ "budget_base": 3 })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recorded_human_edit_names_its_artifact_revision_and_actor() {
    let feature = Feature::review().await;
    feature.drive(written()).await;
    feature.edit("draft.md", "# Draft, edited by hand\n");

    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the edit is approved");

    assert_eq!(
        feature.dir.event_data("working_copy.edit_detected").await,
        [json!({ "artifact": "draft", "revision": 2, "origin": "human", "actor": ACTOR })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approval_records_its_decision_without_a_reason() {
    let feature = Feature::review().await;
    feature.drive(written()).await;

    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the draft is approved");

    assert_eq!(
        feature.dir.event_data("approval.recorded").await,
        [
            json!({ "decision": "approve", "artifact": "draft", "revision": 1, "actor": ACTOR, "reason": null })
        ]
    );
}
