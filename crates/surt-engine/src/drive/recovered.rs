use serde_json::{Map, Value, json};

use super::outcome::{Progress, StepLabel};

pub(super) const DRIVER_RECOVERED: &str = "driver.recovered";

pub(super) fn recovered(previous: Option<u32>, found: &[Progress]) -> Value {
    json!({
        "previous_pid": previous,
        "found": found.iter().map(finding).collect::<Vec<_>>(),
    })
}

fn finding(progress: &Progress) -> Value {
    let (kind, step, details) = match progress {
        Progress::OrphanStopped { step, attempt, pid } => (
            "orphan_stopped",
            step,
            json!({ "attempt": attempt, "pid": pid }),
        ),
        Progress::Discarded { step, attempt } => ("discarded", step, json!({ "attempt": attempt })),
        Progress::InterruptedWithDriver { step, attempt } => (
            "interrupted_with_driver",
            step,
            json!({ "attempt": attempt }),
        ),
        Progress::IntegrationRecorded {
            step,
            branch,
            merge,
        } => (
            "integration_recorded",
            step,
            json!({ "branch": branch, "merge": merge }),
        ),
        Progress::AttemptFailed {
            step,
            attempt,
            code,
            ..
        } => (
            "attempt_failed",
            step,
            json!({ "attempt": attempt, "code": code }),
        ),
        _ => return json!({ "kind": "other" }),
    };
    labelled(kind, step, details)
}

fn labelled(kind: &str, step: &StepLabel, details: Value) -> Value {
    let mut fields = Map::new();
    fields.insert("kind".to_owned(), json!(kind));
    fields.insert("step".to_owned(), json!(step.key));
    fields.insert("item".to_owned(), json!(step.item));
    fields.insert("iteration".to_owned(), json!(step.iteration));
    if let Value::Object(details) = details {
        fields.extend(details);
    }
    Value::Object(fields)
}
