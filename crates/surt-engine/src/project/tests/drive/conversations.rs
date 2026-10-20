use serde_json::{Value, json};

use super::super::fixtures::Catalog;
use super::fixtures::{ACTOR, Feature, PUBLISHER, REVIEW_WORKFLOW, Recording, agent, draft};
use super::stopping::{Interference, Interfering};
use crate::project::DriveOutcome;
use crate::runtime::{Behavior, Delivery, RuntimeRequest, Script, StopLevel};

const CONTINUING_WRITER: &str = r#"
schema = 1
id = "writer"

[inputs.request]
format = "text"

[inputs.draft]
format = "markdown"
optional = true

[inputs.draft-rejection]
format = "text"
optional = true

[outputs.draft]
format = "markdown"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
conversation = "loop"
"#;

const CRASH_EXIT_CODE: i32 = 3;

pub(super) async fn continuing() -> Feature {
    let catalog = Catalog::new()
        .workflow("review", REVIEW_WORKFLOW)
        .agent("writer", agent(CONTINUING_WRITER))
        .agent("publisher", agent(PUBLISHER));
    Feature::new(catalog, "review").await
}

pub(super) async fn rejected_after(feature: &Feature, iteration: u32) {
    let text = format!("# Draft {iteration}\n");
    let outcome = feature
        .drive(Script::new().on("write", iteration, 1, draft(&text)))
        .await
        .outcome;
    assert!(
        matches!(outcome, Ok(DriveOutcome::AtGate { .. })),
        "{outcome:?}"
    );
    reject(feature).await;
}

pub(super) async fn reject(feature: &Feature) {
    feature
        .project
        .reject(&feature.run, None, "shorter", ACTOR)
        .await
        .expect("the rejection is recorded");
}

async fn writes(feature: &Feature) -> Vec<(String, i64, i64, String, Value)> {
    let rows: Vec<(String, i64, i64, String, Option<String>)> = feature
        .dir
        .rows(
            "SELECT e.id, s.iteration, e.attempt, e.state, e.runtime_info FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id WHERE s.step_key = 'write' \
             ORDER BY s.iteration, e.attempt",
        )
        .await;
    rows.into_iter()
        .map(|(id, iteration, attempt, state, info)| {
            let info = info.map_or(Value::Null, |info| {
                serde_json::from_str(&info).expect("runtime_info is JSON")
            });
            (id, iteration, attempt, state, info)
        })
        .collect()
}

pub(super) fn only(requests: Vec<RuntimeRequest>) -> RuntimeRequest {
    let [request] = requests.as_slice() else {
        panic!("one Execution, got {requests:?}");
    };
    request.clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn after_a_rejection_the_next_iteration_continues_the_previous_conversation() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    let runtime = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    feature.drive_with(&runtime).await;

    let request = only(runtime.requests());
    let writes = writes(&feature).await;
    let (first, second) = (&writes[0], &writes[1]);
    let continuation = request.continuation.expect("a continuation");
    assert_eq!(continuation.execution_id, first.0);
    assert_eq!(first.4["session_id"], continuation.session_id.to_string());
    assert_ne!(continuation.session_id, request.session_id);
    assert_eq!(first.4["conversation"], json!({ "fresh": "no_previous" }));
    assert_eq!(
        second.4["conversation"],
        json!({ "continued_from": first.0 })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conversation_continues_the_latest_successful_execution_of_the_loop() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    rejected_after(&feature, 2).await;
    let runtime = Recording::new(Script::new().on("write", 3, 1, draft("# Draft 3\n")));

    feature.drive_with(&runtime).await;

    let continuation = only(runtime.requests())
        .continuation
        .expect("a continuation");
    let writes = writes(&feature).await;
    assert_eq!(continuation.execution_id, writes[1].0);
}

#[tokio::test(flavor = "multi_thread")]
async fn interrupted_and_failed_executions_are_never_continued() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    Interfering::new(
        Script::new().on("write", 2, 1, Behavior::WaitUntilStopped),
        "write",
        Interference::Stop(StopLevel::Graceful),
    )
    .drive(&feature)
    .await;
    let crash = Behavior::Crash {
        exit_code: CRASH_EXIT_CODE,
    };
    let runtime = Recording::new(Script::new().on("write", 2, 2, crash).on(
        "write",
        2,
        3,
        draft("# Draft 2\n"),
    ));

    feature.drive_with(&runtime).await;

    let writes = writes(&feature).await;
    let states: Vec<&str> = writes.iter().map(|write| write.3.as_str()).collect();
    assert_eq!(states, ["succeeded", "interrupted", "failed", "succeeded"]);
    for request in runtime.requests() {
        let continuation = request.continuation.expect("a continuation");
        assert_eq!(continuation.execution_id, writes[0].0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lost_transcript_restarts_in_the_same_execution_without_spending_an_attempt() {
    let feature = continuing().await;
    rejected_after(&feature, 1).await;
    let lost = Behavior::TranscriptMissing(Delivery::new("done").output(
        "draft",
        "draft.md",
        "# Draft 2\n",
    ));

    let driven = feature.drive(Script::new().on("write", 2, 1, lost)).await;

    assert!(
        matches!(driven.outcome, Ok(DriveOutcome::AtGate { .. })),
        "{:?}",
        driven.outcome
    );
    let writes = writes(&feature).await;
    assert_eq!(writes.len(), 2);
    let restarted = &writes[1];
    assert_eq!((restarted.1, restarted.2), (2, 1));
    assert_eq!(restarted.3, "succeeded");
    assert_eq!(
        restarted.4["conversation"],
        json!({ "restarted": "transcript_missing", "previous": writes[0].0 })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_without_a_conversation_always_starts_fresh() {
    let feature = Feature::review().await;
    rejected_after(&feature, 1).await;
    let runtime = Recording::new(Script::new().on("write", 2, 1, draft("# Draft 2\n")));

    feature.drive_with(&runtime).await;

    assert!(only(runtime.requests()).continuation.is_none());
    let writes = writes(&feature).await;
    for write in &writes {
        assert_eq!(write.4.get("conversation"), None, "{}", write.4);
    }
}
