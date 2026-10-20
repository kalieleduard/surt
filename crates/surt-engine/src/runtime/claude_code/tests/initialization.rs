use serde_json::{Value, json};
use uuid::Uuid;

use super::executions::Execution;
use super::streams::{Fixture, Stream};
use crate::runtime::claude_code::transcript::Observation;
use crate::runtime::claude_code::transcript::Transcript;
use crate::runtime::{Exit, FailureClass, RuntimeError, Verdict};

fn delivered_note() -> Stream {
    Stream::captured(Fixture::Success)
        .with_envelope(json!({ "outcome": "done", "outputs": { "note": "note.md" } }))
}

fn initialized_with(field: &str, value: Value) -> RuntimeError {
    let stream = delivered_note().with_init(field, value);
    let execution = Execution::new(stream.session_id());
    execution.deliver_note();
    execution.failure(&stream, Exit::Code(0))
}

fn assert_class(error: &RuntimeError, class: FailureClass, code: &str) {
    assert_eq!((error.class, error.code.as_str()), (class, code), "{error}");
}

#[test]
fn an_initialization_other_than_requested_asks_for_a_stop() {
    let stream = delivered_note().with_init("model", json!("claude-opus-5-5"));
    let execution = Execution::new(stream.session_id());
    let mut transcript = Transcript::new(execution.invocation(&[]).expectation);

    let observations: Vec<Observation> = stream
        .lines()
        .iter()
        .map(|line| transcript.observe(line.as_bytes()))
        .collect();

    assert!(observations.contains(&Observation::Incompatible));
    assert!(
        !observations
            .iter()
            .any(|observation| matches!(observation, Observation::Initialized(_)))
    );
}

#[test]
fn a_matching_initialization_is_reported_once() {
    let stream = delivered_note();
    let mut lines = stream.lines();
    lines.push(lines[1].clone());
    let execution = Execution::new(stream.session_id());
    let mut transcript = Transcript::new(execution.invocation(&[]).expectation);

    let initialized: Vec<Observation> = lines
        .iter()
        .map(|line| transcript.observe(line.as_bytes()))
        .filter(|observation| matches!(observation, Observation::Initialized(_)))
        .collect();

    let [Observation::Initialized(initialization)] = initialized.as_slice() else {
        panic!("expected one initialization, got {initialized:?}");
    };
    assert_eq!(initialization.session_id, stream.session_id());
    assert_eq!(initialization.model, "claude-haiku-5-5");
}

#[test]
fn another_model_is_not_retryable() {
    let error = initialized_with("model", json!("claude-opus-5-5"));

    assert_class(&error, FailureClass::NotRetryable, "model_mismatch");
}

#[test]
fn another_session_is_retryable() {
    let error = initialized_with("session_id", json!(Uuid::now_v7().to_string()));

    assert_class(&error, FailureClass::Retryable, "session_mismatch");
}

#[test]
fn a_version_older_than_the_floor_or_unreadable_is_not_supported() {
    for version in [
        json!("2.1.300"),
        json!("2.0.999"),
        json!("two"),
        json!(null),
    ] {
        let error = initialized_with("claude_code_version", version);

        assert_class(&error, FailureClass::NotRetryable, "unsupported_version");
    }
}

#[test]
fn newer_versions_are_accepted() {
    for version in ["2.1.301", "2.1.302", "2.2.0", "3.0.0-beta.1"] {
        let stream = delivered_note().with_init("claude_code_version", json!(version));
        let execution = Execution::new(stream.session_id());
        execution.deliver_note();

        let completion = execution
            .replay(&stream, Exit::Code(0))
            .expect("a completion");

        assert!(
            matches!(execution.verdict(&completion), Verdict::Accepted(_)),
            "{version}"
        );
    }
}

#[test]
fn another_permission_mode_tool_set_mcp_server_or_auth_source_is_not_retryable() {
    let mismatches = [
        ("permissionMode", json!("default")),
        (
            "tools",
            json!(["Read", "StructuredOutput", "Write", "Bash"]),
        ),
        ("tools", json!(["Read", "Write"])),
        ("tools", json!(null)),
        (
            "mcp_servers",
            json!([{ "name": "github", "status": "connected" }]),
        ),
        ("apiKeySource", json!("ANTHROPIC_API_KEY")),
    ];
    for (field, value) in mismatches {
        let error = initialized_with(field, value);

        assert_class(
            &error,
            FailureClass::NotRetryable,
            "initialization_mismatch",
        );
    }
}
