use serde_json::json;
use uuid::Uuid;

use super::executions::{Execution, PREVIOUS_EXECUTION};
use super::streams::{Fixture, Stream, delivered_note};
use crate::runtime::claude_code::launcher::Launched;
use crate::runtime::claude_code::restart;
use crate::runtime::{Conversation, Exit, FailureClass, RestartReason, StopSignal};

fn continued(execution: &Execution, stream: &Stream, exit: Exit) -> Launched {
    Launched {
        transcript: execution.transcript(&stream.lines()),
        exit,
    }
}

fn restarted(reason: RestartReason) -> Option<Conversation> {
    Some(Conversation::Restarted {
        reason,
        previous: PREVIOUS_EXECUTION.to_owned(),
    })
}

fn decided(execution: &Execution, stream: &Stream, exit: Exit) -> Option<Conversation> {
    let launched = continued(execution, stream, exit);
    restart::needed(&execution.request, &launched, &StopSignal::new())
}

#[test]
fn the_probed_resume_without_a_transcript_restarts_as_transcript_missing() {
    let fixture = Fixture::ResumeWithoutTranscript;
    let stream = Stream::captured(fixture);
    let execution = Execution::new(Uuid::now_v7()).continuing();

    let decision = decided(&execution, &stream, fixture.exit());

    assert_eq!(decision, restarted(RestartReason::TranscriptMissing));
    assert_eq!(
        decision.map(|conversation| conversation.value()),
        Some(json!({ "restarted": "transcript_missing", "previous": PREVIOUS_EXECUTION }))
    );
}

#[test]
fn a_continued_launch_ending_uninitialized_for_another_reason_restarts_as_refused() {
    let execution = Execution::new(Uuid::now_v7()).continuing();
    let refused = Stream::captured(Fixture::ResumeWithoutTranscript)
        .with_result("errors", json!(["--fork-session is not supported"]));
    let silent = Stream::captured(Fixture::ResumeWithoutTranscript).without_result();

    let decisions = [
        decided(&execution, &refused, Exit::Code(1)),
        decided(&execution, &silent, Exit::Code(1)),
    ];

    let expected = restarted(RestartReason::ContinuationRefused);
    assert_eq!(decisions, [expected.clone(), expected]);
}

#[test]
fn a_continued_launch_that_initialized_is_never_restarted_and_fails_as_today() {
    let fixture = Fixture::AuthExpired;
    let stream = Stream::captured(fixture);
    let execution = Execution::new(stream.session_id()).continuing();
    let incompatible = delivered_note().with_init("model", json!("claude-opus-5-5"));

    let decision = decided(&execution, &stream, fixture.exit());

    assert_eq!(decision, None);
    assert_eq!(decided(&execution, &incompatible, Exit::Code(0)), None);
    let error = execution.failure(&stream, fixture.exit());
    assert_eq!(error.class, FailureClass::NotRetryable);
    assert_eq!(error.code, "authentication_failed");
}

#[test]
fn a_stop_during_the_continued_launch_never_restarts_it() {
    let fixture = Fixture::ResumeWithoutTranscript;
    let stream = Stream::captured(fixture);
    let execution = Execution::new(Uuid::now_v7()).continuing();
    let stop = StopSignal::new();
    stop.graceful();
    let ended = continued(&execution, &stream, fixture.exit());

    assert_eq!(decided(&execution, &stream, Exit::Stopped), None);
    assert_eq!(restart::needed(&execution.request, &ended, &stop), None);
}

#[test]
fn a_launch_without_a_continuation_is_never_restarted() {
    let fixture = Fixture::ResumeWithoutTranscript;
    let stream = Stream::captured(fixture);
    let execution = Execution::new(Uuid::now_v7()).in_a_loop();

    assert_eq!(decided(&execution, &stream, fixture.exit()), None);
}
