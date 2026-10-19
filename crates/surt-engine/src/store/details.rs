use serde_json::{Value, json};
use sqlx::SqliteConnection;

use super::approvals::{Decision, NewDecision};
use super::executions::ExecutionKind;
use super::runs::NewRunRows;

const BASE_ARTIFACT: &str = "base";

pub(super) fn run_created(rows: &NewRunRows) -> Value {
    let base = rows
        .initial_artifacts
        .iter()
        .find(|artifact| artifact.name == BASE_ARTIFACT)
        .and_then(|artifact| artifact.files.first())
        .and_then(|(_, bytes)| serde_json::from_slice::<Value>(bytes).ok());
    json!({ "base": base, "actor": rows.actor })
}

pub(super) struct ApprovalDetails<'a> {
    pub(super) decision: &'a str,
    pub(super) artifact: &'a str,
    pub(super) revision: u32,
    pub(super) actor: &'a str,
    pub(super) reason: Option<&'a str>,
}

pub(super) fn approval_recorded(details: &ApprovalDetails<'_>) -> Value {
    let reason = details
        .reason
        .map(|reason| reason.lines().next().unwrap_or_default());
    json!({
        "decision": details.decision,
        "artifact": details.artifact,
        "revision": details.revision,
        "actor": details.actor,
        "reason": reason,
    })
}

pub(super) fn decision_recorded(new: &NewDecision<'_>, revision: u32) -> Value {
    let (decision, reason) = match new.decision {
        Decision::Approve => ("approve", None),
        Decision::Reject { reason, .. } => ("reject", Some(reason)),
    };
    approval_recorded(&ApprovalDetails {
        decision,
        artifact: new.artifact,
        revision,
        actor: new.actor,
        reason,
    })
}

pub(super) async fn iteration_started(
    connection: &mut SqliteConnection,
    loop_step: &str,
    iteration: i64,
    until: &str,
) -> Result<Value, sqlx::Error> {
    if iteration <= 1 {
        return Ok(json!({ "iteration": iteration, "cause": "loop_started" }));
    }
    let outcome: Option<Option<String>> = sqlx::query_scalar(
        "SELECT outcome FROM step_run WHERE loop_step_run_id = ? AND iteration = ? AND step_key = ?",
    )
    .bind(loop_step)
    .bind(iteration - 1)
    .bind(until)
    .fetch_optional(connection)
    .await?;
    Ok(json!({
        "iteration": iteration,
        "cause": "until_not_met",
        "step": until,
        "outcome": outcome.flatten(),
    }))
}

pub(super) fn execution_started(attempt: i64, kind: &ExecutionKind<'_>) -> Value {
    match kind {
        ExecutionKind::Agent { slot, runtime, .. } => {
            json!({ "attempt": attempt, "slot": slot, "runtime": runtime })
        }
        ExecutionKind::Operation { id } => json!({ "attempt": attempt, "operation": id }),
    }
}
