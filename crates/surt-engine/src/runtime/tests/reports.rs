use serde_json::json;

use super::fixtures::{Execution, MODEL, valid};
use crate::runtime::Behavior;

const CRASH_EXIT_CODE: i32 = 137;
const REPORTED_MODEL: &str = "claude-haiku-5-5";

#[tokio::test]
async fn a_delivery_reports_its_session_model_and_exit_code() {
    let execution = Execution::new();

    let report = execution
        .report(Behavior::Deliver(valid("approved")))
        .await
        .expect("a report");

    assert_eq!(
        report.fields(),
        json!({
            "session_id": execution.request.session_id.to_string(),
            "model": MODEL,
            "exit_code": 0
        })
    );
}

#[tokio::test]
async fn a_mismatched_model_is_reported_as_the_fake_named_it() {
    let behavior = Behavior::ModelMismatch {
        reported: REPORTED_MODEL.to_owned(),
        delivery: valid("approved"),
    };

    let report = Execution::new().report(behavior).await.expect("a report");

    assert_eq!(report.model.as_deref(), Some(REPORTED_MODEL));
}

#[tokio::test]
async fn a_crash_reports_its_exit_code() {
    let crash = Behavior::Crash {
        exit_code: CRASH_EXIT_CODE,
    };

    let report = Execution::new().report(crash).await.expect("a report");

    assert_eq!(report.exit_code, Some(CRASH_EXIT_CODE));
}

#[tokio::test]
async fn a_limit_failure_still_reports_what_was_known_without_an_exit_code() {
    let execution = Execution::new();
    let limited = Behavior::LimitFailure { resets_at: None };

    let report = execution.report(limited).await.expect("a report");

    assert_eq!(
        report.fields(),
        json!({
            "session_id": execution.request.session_id.to_string(),
            "model": MODEL
        })
    );
}
