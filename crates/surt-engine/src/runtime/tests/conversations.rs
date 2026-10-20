use std::fs;

use serde_json::{Value, json};
use uuid::Uuid;

use super::fixtures::{Execution, REPORT, valid};
use crate::runtime::{
    Behavior, Continuation, Conversation, FreshReason, RestartReason, STREAM_LOG, Script,
    StopSignal, Verdict,
};
use crate::workflow::ConversationScope;

const FULL: &str = "## Inputs\n- request rev 1\n- draft rev 1\n";
const DELTA: &str = "## Inputs\n- draft-rejection rev 1\n";
const PREVIOUS: &str = "0192f3a8-7b1c-7000-8000-000000000000";

fn continuing() -> (Execution, Uuid) {
    let mut execution = Execution::new();
    let session_id = Uuid::now_v7();
    execution.request.launch.profile.conversation = ConversationScope::Loop;
    execution.request.context = FULL.to_owned();
    execution.request.continuation = Some(Continuation {
        session_id,
        execution_id: PREVIOUS.to_owned(),
        context: DELTA.to_owned(),
    });
    (execution, session_id)
}

fn fed(execution: &Execution) -> Value {
    let log = fs::read_to_string(execution.request.log_dir.join(STREAM_LOG)).expect("the stream");
    let first = log.lines().next().expect("a first line");
    serde_json::from_str(first).expect("a JSON line")
}

#[tokio::test]
async fn a_continued_execution_resumes_the_previous_session_with_the_delta_context() {
    let (execution, session_id) = continuing();

    let report = execution
        .report(Behavior::Deliver(valid("approved")))
        .await
        .expect("a report");

    assert_eq!(
        report.conversation,
        Some(Conversation::ContinuedFrom(PREVIOUS.to_owned()))
    );
    assert_eq!(
        report.fields()["conversation"],
        json!({ "continued_from": PREVIOUS })
    );
    assert_eq!(
        fed(&execution),
        json!({ "type": "fake", "resumed": session_id.to_string(), "stdin": DELTA })
    );
}

#[tokio::test]
async fn a_lost_transcript_restarts_fresh_with_the_full_context_and_still_delivers() {
    let (execution, _) = continuing();

    let report = execution
        .report(Behavior::TranscriptMissing(valid("approved")))
        .await
        .expect("a report");
    let verdict = execution
        .run(Behavior::TranscriptMissing(valid("approved")))
        .await
        .expect("the fake completes");

    assert_eq!(
        report.fields()["conversation"],
        json!({ "restarted": "transcript_missing", "previous": PREVIOUS })
    );
    assert_eq!(
        report.conversation,
        Some(Conversation::Restarted {
            reason: RestartReason::TranscriptMissing,
            previous: PREVIOUS.to_owned(),
        })
    );
    assert_eq!(
        fed(&execution),
        json!({ "type": "fake", "resumed": null, "stdin": FULL })
    );
    assert!(matches!(verdict, Verdict::Accepted(_)), "{verdict:?}");
}

#[tokio::test]
async fn a_scripted_transcript_missing_restarts_like_the_rust_behaviour() {
    let (execution, _) = continuing();
    let script = Script::from_toml(&format!(
        r#"
[[on]]
step = "write"
transcript_missing = {{ outcome = "approved", outputs = {{ report = "report.md" }}, files = {{ "report.md" = {REPORT:?} }} }}
"#
    ))
    .expect("a valid script");

    let completion = execution
        .complete_script(script, StopSignal::new())
        .await
        .expect("the fake completes");

    assert_eq!(completion.result.map(|result| result.succeeded), Some(true));
    assert_eq!(fed(&execution)["stdin"], FULL);
}

#[tokio::test]
async fn a_first_execution_of_a_loop_agent_reports_that_it_had_nothing_to_continue() {
    let mut execution = Execution::new();
    execution.request.launch.profile.conversation = ConversationScope::Loop;

    let report = execution
        .report(Behavior::TranscriptMissing(valid("approved")))
        .await
        .expect("a report");

    assert_eq!(
        report.conversation,
        Some(Conversation::Fresh(FreshReason::NoPrevious))
    );
    assert_eq!(
        report.fields()["conversation"],
        json!({ "fresh": "no_previous" })
    );
}

#[tokio::test]
async fn an_agent_without_a_conversation_reports_none() {
    let execution = Execution::new();

    let report = execution
        .report(Behavior::Deliver(valid("approved")))
        .await
        .expect("a report");

    assert_eq!(report.conversation, None);
    assert_eq!(report.fields().get("conversation"), None);
}
