use serde_json::json;

use super::executions::{Execution, NOTE};
use super::streams::{Fixture, Stream};
use crate::artifacts::DeliveryRejection;
use crate::runtime::{Exit, FailureClass, Rejection, RuntimeCompletion, Verdict};

const SIGTERM_EXIT_CODE: i32 = 143;

fn delivered_note() -> Stream {
    Stream::captured(Fixture::Success)
        .with_envelope(json!({ "outcome": "done", "outputs": { "note": "note.md" } }))
}

fn complete(execution: &Execution, stream: &Stream, exit: Exit) -> RuntimeCompletion {
    execution.replay(stream, exit).expect("a completion")
}

fn rejection(stream: &Stream, exit: Exit) -> Rejection {
    let execution = Execution::new(stream.session_id());
    execution.deliver_note();
    let verdict = execution.verdict(&complete(&execution, stream, exit));
    let Verdict::Rejected(rejection) = verdict else {
        panic!("expected a rejection, got {verdict:?}");
    };
    rejection
}

#[test]
fn a_captured_success_whose_files_were_delivered_is_accepted() {
    let stream = delivered_note();
    let execution = Execution::new(stream.session_id());
    execution.deliver_note();

    let completion = complete(&execution, &stream, Fixture::Success.exit());

    let Verdict::Accepted(delivery) = execution.verdict(&completion) else {
        panic!("expected an accepted delivery");
    };
    assert_eq!(delivery.outcome, "done");
    assert_eq!(delivery.outputs["note"].files[0].bytes, NOTE.as_bytes());
}

#[test]
fn unknown_events_and_fields_anywhere_in_the_stream_are_ignored() {
    let stream = delivered_note()
        .first(json!({ "type": "plugin_install", "status": "started" }))
        .with_init("brand_new_field", json!({ "nested": true }))
        .before_result(json!({ "type": "system", "subtype": "brand_new", "uuid": "x" }))
        .with_result("brand_new_total", json!(7))
        .last(json!({ "type": "system", "subtype": "hook_response", "output": "" }));
    let execution = Execution::new(stream.session_id());
    execution.deliver_note();

    let completion = complete(&execution, &stream, Exit::Code(0));

    assert!(matches!(
        execution.verdict(&completion),
        Verdict::Accepted(_)
    ));
}

#[test]
fn a_schema_valid_envelope_without_its_files_is_rejected() {
    for fixture in [
        Fixture::Success,
        Fixture::SchemaMiss,
        Fixture::PermissionDenied,
    ] {
        let stream = Stream::captured(fixture);
        let execution = Execution::new(stream.session_id());

        let verdict = execution.verdict(&complete(&execution, &stream, fixture.exit()));

        let Verdict::Rejected(Rejection::Delivery(_)) = verdict else {
            panic!("{fixture:?}: expected a rejected delivery, got {verdict:?}");
        };
    }
}

#[test]
fn a_success_subtype_reporting_an_error_is_not_accepted() {
    let stream = delivered_note().with_result("is_error", json!(true));
    let execution = Execution::new(stream.session_id());
    execution.deliver_note();

    let error = execution
        .replay(&stream, Exit::Code(0))
        .expect_err("an unsuccessful result");

    assert_eq!(error.class, FailureClass::Retryable);
    assert_eq!(error.code, "unsuccessful_result");
}

#[test]
fn a_success_without_structured_output_has_no_envelope() {
    let stream = delivered_note().with_envelope(json!(null));

    assert_eq!(
        rejection(&stream, Exit::Code(0)),
        Rejection::Delivery(DeliveryRejection::MissingEnvelope)
    );
}

#[test]
fn a_success_shaped_stream_needs_an_initialization_and_a_zero_exit() {
    assert_eq!(
        rejection(&delivered_note().without_init(), Exit::Code(0)),
        Rejection::MissingInitialization
    );
    assert_eq!(
        rejection(&delivered_note(), Exit::Code(1)),
        Rejection::ExitCode(1)
    );
}

#[test]
fn a_signalled_exit_without_a_result_is_a_retryable_crash() {
    let stream = Stream::captured(Fixture::Sigterm);
    assert_eq!(Fixture::Sigterm.exit(), Exit::Code(SIGTERM_EXIT_CODE));

    let rejection = rejection(&stream, Fixture::Sigterm.exit());

    assert_eq!(rejection, Rejection::ExitCode(SIGTERM_EXIT_CODE));
    assert_eq!(rejection.class(), FailureClass::Retryable);
}

#[test]
fn a_stream_cut_by_a_requested_stop_is_interrupted() {
    let stream = Stream::captured(Fixture::Sigterm);
    let execution = Execution::new(stream.session_id());

    let completion = complete(&execution, &stream, Exit::Stopped);

    assert!(completion.result.is_none());
    assert!(matches!(
        execution.verdict(&completion),
        Verdict::Interrupted
    ));
}
