use serde_json::{Value, json};
use uuid::Uuid;

use super::executions::Execution;
use super::streams::{Fixture, Stream};
use crate::runtime::claude_code::launched_fresh;
use crate::runtime::{Continuation, Conversation, Exit, FreshReason};
use crate::workflow::ConversationScope;

fn reported(fixture: Fixture, exit: Exit) -> Value {
    let stream = Stream::captured(fixture);
    Execution::new(stream.session_id())
        .report(&stream, exit)
        .fields()
}

#[test]
fn a_success_reports_identity_turns_usage_cost_and_denials() {
    let report = reported(Fixture::Success, Fixture::Success.exit());

    assert_eq!(
        report,
        json!({
            "session_id": "e31e4cde-acd7-481b-b29f-9ee8fae1901a",
            "model": "claude-haiku-5-5",
            "effort": "low",
            "exit_code": 0,
            "turns": 3,
            "usage": {
                "input_tokens": 4,
                "output_tokens": 346,
                "cache_creation_input_tokens": 615,
                "cache_read_input_tokens": 8526
            },
            "cost_usd": 0.000_381_660_000_000_000_03,
            "permission_denials": []
        })
    );
}

#[test]
fn an_expired_login_reports_what_the_failed_result_carried() {
    let report = reported(Fixture::AuthExpired, Fixture::AuthExpired.exit());

    assert_eq!(
        report,
        json!({
            "session_id": "6409d680-05bb-47ba-9908-16a0386fa485",
            "model": "claude-haiku-5-5",
            "effort": "low",
            "exit_code": 1,
            "turns": 1,
            "usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            },
            "cost_usd": 0.0,
            "permission_denials": []
        })
    );
}

#[test]
fn a_nonexistent_model_is_reported_as_the_runtime_named_it() {
    let report = reported(Fixture::BadModel, Fixture::BadModel.exit());

    assert_eq!(report["model"], "claude-nonexistent-0-0");
    assert_eq!(report["session_id"], "45c2ea83-2948-473b-8e9c-4735b6150fd7");
    assert_eq!(report["exit_code"], 1);
}

#[test]
fn a_process_killed_before_its_result_reports_only_what_was_known() {
    let report = reported(Fixture::Sigterm, Fixture::Sigterm.exit());

    assert_eq!(
        report,
        json!({
            "session_id": "9298fb31-5f5f-42a2-8af0-66ec19bc85fa",
            "model": "claude-haiku-5-5",
            "effort": "low",
            "exit_code": 143
        })
    );
}

#[test]
fn a_stopped_process_has_no_exit_code() {
    let report = reported(Fixture::Sigterm, Exit::Stopped);

    assert_eq!(report.get("exit_code"), None);
    assert_eq!(report["session_id"], "9298fb31-5f5f-42a2-8af0-66ec19bc85fa");
}

#[test]
fn a_denied_tool_is_reported_by_name_without_its_input() {
    let report = reported(Fixture::PermissionDenied, Fixture::PermissionDenied.exit());

    assert_eq!(
        report["permission_denials"],
        json!([{
            "tool_name": "Write",
            "tool_use_id": "toolu_01GJCQkNisG7uD9GYaJjKQUc"
        }])
    );
}

#[test]
fn a_stream_without_initialization_takes_the_session_from_the_result() {
    let stream = Stream::captured(Fixture::Success).without_init();

    let report = Execution::new(stream.session_id())
        .report(&stream, Exit::Code(0))
        .fields();

    assert_eq!(report["session_id"], "e31e4cde-acd7-481b-b29f-9ee8fae1901a");
    assert_eq!(report.get("model"), None);
}

#[test]
fn values_the_result_left_out_are_absent_not_zero() {
    let stream = Stream::captured(Fixture::Success)
        .with_result("num_turns", Value::Null)
        .with_result("total_cost_usd", json!("unknown"))
        .with_result("usage", json!({ "output_tokens": 7 }))
        .with_result("permission_denials", Value::Null);

    let report = Execution::new(stream.session_id())
        .report(&stream, Exit::Code(0))
        .fields();

    assert_eq!(report.get("turns"), None);
    assert_eq!(report.get("cost_usd"), None);
    assert_eq!(report.get("permission_denials"), None);
    assert_eq!(report["usage"], json!({ "output_tokens": 7 }));
}

#[test]
fn a_continuation_is_launched_fresh_and_reported_as_unsupported_until_sessions_fork() {
    let mut execution = Execution::new(Uuid::now_v7());
    execution.request.launch.profile.conversation = ConversationScope::Loop;
    let fresh = launched_fresh(&execution.request);
    execution.request.continuation = Some(Continuation {
        session_id: Uuid::now_v7(),
        execution_id: "previous".to_owned(),
        context: "delta".to_owned(),
    });

    let continued = launched_fresh(&execution.request);

    assert_eq!(fresh, Some(Conversation::Fresh(FreshReason::NoPrevious)));
    assert_eq!(
        continued,
        Some(Conversation::Fresh(FreshReason::ContinuationUnsupported))
    );
}
