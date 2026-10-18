use serde_json::json;

use super::executions::Execution;
use super::streams::{Fixture, Stream};
use crate::runtime::{Exit, FailureClass, RuntimeError};

const RESETS_AT: i64 = 1_793_491_200;
const LATER_RESET: i64 = 1_793_500_000;

fn failure_of(stream: &Stream, exit: Exit) -> RuntimeError {
    Execution::new(stream.session_id()).failure(stream, exit)
}

fn api_failure(category: &str) -> Stream {
    Stream::captured(Fixture::AuthExpired).with_assistant_error(category)
}

fn assert_class(error: &RuntimeError, class: FailureClass, code: &str) {
    assert_eq!((error.class, error.code.as_str()), (class, code), "{error}");
}

#[test]
fn an_expired_login_is_not_retryable() {
    let stream = Stream::captured(Fixture::AuthExpired);

    let error = failure_of(&stream, Fixture::AuthExpired.exit());

    assert_class(&error, FailureClass::NotRetryable, "authentication_failed");
    assert_eq!(
        error.message,
        "Failed to authenticate: OAuth session expired and could not be refreshed"
    );
    assert_eq!(error.resets_at, None);
}

#[test]
fn a_nonexistent_model_passes_initialization_and_is_not_retryable() {
    let stream = Stream::captured(Fixture::BadModel);
    let execution = Execution::new(stream.session_id()).with_model("claude-nonexistent-0-0");

    let error = execution.failure(&stream, Fixture::BadModel.exit());

    assert_class(&error, FailureClass::NotRetryable, "model_not_found");
    assert!(error.message.contains("claude-nonexistent-0-0"), "{error}");
}

#[test]
fn account_and_request_errors_are_not_retryable() {
    for category in ["billing_error", "account_on_hold", "invalid_request"] {
        let error = failure_of(&api_failure(category), Exit::Code(1));

        assert_class(&error, FailureClass::NotRetryable, category);
    }
}

#[test]
fn a_rate_limit_is_transient_and_carries_the_reported_reset() {
    let stream = api_failure("rate_limit")
        .before_result(json!({
            "type": "rate_limit_event",
            "rate_limit_info": { "status": "rejected", "resetsAt": RESETS_AT }
        }))
        .before_result(json!({
            "type": "rate_limit_event",
            "rate_limit_info": { "status": "allowed", "resetsAt": LATER_RESET }
        }));

    let error = failure_of(&stream, Exit::Code(1));

    assert_class(&error, FailureClass::Transient, "rate_limit");
    assert_eq!(error.resets_at, Some(RESETS_AT));
}

#[test]
fn a_rate_limit_without_a_reported_reset_has_none() {
    let error = failure_of(&api_failure("rate_limit"), Exit::Code(1));

    assert_class(&error, FailureClass::Transient, "rate_limit");
    assert_eq!(error.resets_at, None);
}

#[test]
fn overload_and_server_errors_are_transient() {
    for category in ["overloaded", "server_error"] {
        let error = failure_of(&api_failure(category), Exit::Code(1));

        assert_class(&error, FailureClass::Transient, category);
        assert_eq!(error.resets_at, None);
    }
}

#[test]
fn a_lost_connection_is_a_transient_network_failure() {
    let retry = json!({
        "type": "system", "subtype": "api_retry", "attempt": 10,
        "max_retries": 10, "retry_delay_ms": 0, "error_status": null, "error": "unknown"
    });
    for stream in [
        api_failure("unknown").before_result(retry.clone()),
        Stream::captured(Fixture::Sigterm).last(retry.clone()),
    ] {
        let error = failure_of(&stream, Exit::Code(1));

        assert_class(&error, FailureClass::Transient, "network");
    }
}

#[test]
fn an_unknown_error_category_is_retryable() {
    let error = failure_of(&api_failure("brand_new_category"), Exit::Code(1));

    assert_class(&error, FailureClass::Retryable, "brand_new_category");
}

#[test]
fn a_retried_http_error_is_classified_by_its_category() {
    let stream = api_failure("authentication_failed").before_result(json!({
        "type": "system", "subtype": "api_retry", "error_status": 401, "error": "authentication_failed"
    }));

    let error = failure_of(&stream, Exit::Code(1));

    assert_class(&error, FailureClass::NotRetryable, "authentication_failed");
}

#[test]
fn a_reached_turn_limit_is_retryable() {
    let stream = Stream::captured(Fixture::MaxTurns);

    let error = failure_of(&stream, Fixture::MaxTurns.exit());

    assert_class(&error, FailureClass::Retryable, "max_turns");
}

#[test]
fn an_error_the_run_recovered_from_does_not_fail_it() {
    let stream = Stream::captured(Fixture::Success).before_result(json!({
        "type": "assistant", "error": "max_output_tokens",
        "message": { "content": [{ "type": "text", "text": "truncated" }] }
    }));
    let execution = Execution::new(stream.session_id());

    let completion = execution
        .replay(&stream, Exit::Code(0))
        .expect("a completion");

    assert!(completion.result.is_some_and(|result| result.succeeded));
}

#[test]
fn a_line_that_is_not_json_is_a_protocol_failure() {
    let stream = Stream::captured(Fixture::Success);
    let mut lines = stream.lines();
    lines.insert(2, "{\"type\": \"assistant\", truncated".to_owned());
    lines.insert(3, String::new());

    let error = Execution::new(stream.session_id())
        .replay_lines(&lines, Exit::Code(0))
        .expect_err("a protocol failure");

    assert_class(&error, FailureClass::Retryable, "protocol_error");
}

#[test]
fn a_second_result_is_a_protocol_failure() {
    let stream = Stream::captured(Fixture::Success);
    let mut lines = stream.lines();
    lines.push(lines[lines.len() - 1].clone());

    let error = Execution::new(stream.session_id())
        .replay_lines(&lines, Exit::Code(0))
        .expect_err("a protocol failure");

    assert_class(&error, FailureClass::Retryable, "protocol_error");
}
