use serde_json::Value;
use surt_engine::project::{
    BlockReason, BlockedStep, Conflict, GateStatus, GateView, ItemState, ItemStatus, LastFailure,
    RunStatus,
};

use super::detail::merge_into_base;
use super::{resume_command, state_name, state_names};
use crate::json::Fields;
use crate::show::origin;

pub(super) fn features(statuses: &[RunStatus]) -> Fields {
    let features: Vec<Value> = statuses
        .iter()
        .map(|status| feature(status).into())
        .collect();
    Fields::new().with("features", features)
}

pub(super) fn feature(status: &RunStatus) -> Fields {
    let resume = resume_command(status);
    let states: Vec<&str> = status
        .states
        .iter()
        .map(|state| state_name(*state))
        .collect();
    let fields = Fields::new()
        .with("name", status.subject.as_str())
        .with("run", status.run)
        .with("runs", status.runs)
        .with("state", state_names(&status.states))
        .with("states", states)
        .time("ended_at", status.ended_at)
        .maybe(
            "driver",
            status.driver.map(|driver| {
                Fields::new()
                    .with("pid", driver.pid)
                    .time("since", driver.since)
            }),
        );
    work(repository(fields, status), status, &resume)
}

fn repository(fields: Fields, status: &RunStatus) -> Fields {
    fields
        .maybe(
            "base",
            status.base.as_ref().map(|base| {
                Fields::new()
                    .with("sha", base.sha.as_str())
                    .maybe("branch", base.branch.as_deref())
            }),
        )
        .maybe(
            "branch",
            status.branch.as_ref().map(|branch| {
                Fields::new()
                    .with("name", branch.name.as_str())
                    .with("tip", branch.tip.as_str())
            }),
        )
        .with("merge_into_base", merge_into_base(status.merge_into_base))
}

fn work(fields: Fields, status: &RunStatus, resume: &str) -> Fields {
    let tasks: Vec<Value> = status
        .items
        .iter()
        .map(|item| task(item, status, resume))
        .collect();
    fields
        .with(
            "agent_slots",
            Fields::new()
                .with("in_use", status.slots.in_use)
                .with("total", status.slots.total),
        )
        .with("gates", status.gates.iter().map(gate).collect::<Vec<_>>())
        .with("tasks", tasks)
        .list("blocked", status.blocked.iter().map(blocked).collect())
        .with("resume", resume)
}

fn gate(view: &GateView) -> Value {
    let (status, decision) = match view.status {
        GateStatus::NotReached => ("not_reached", None),
        GateStatus::Waiting { .. } => ("waiting", None),
        GateStatus::Approved { .. } => ("approved", Some("approve")),
        GateStatus::Rejected { .. } => ("rejected", Some("reject")),
    };
    Fields::new()
        .with("step", view.step.as_str())
        .with("artifact", view.artifact.as_str())
        .with("status", status)
        .maybe(
            "revision",
            view.revision.as_ref().map(|revision| {
                Fields::new()
                    .with("number", revision.number)
                    .with("origin", origin(revision.origin))
                    .with("sha256", revision.sha256.as_str())
                    .with("files", revision.files.clone())
            }),
        )
        .maybe(
            "approval",
            decision.map(|decision| {
                Fields::new()
                    .with("decision", decision)
                    .maybe("actor", view.decided_by.as_deref())
                    .time("decided_at", view.decided_at)
            }),
        )
        .list("edited", view.edited.clone())
        .list("missing", view.missing.clone())
        .into()
}

fn task(item: &ItemStatus, status: &RunStatus, resume: &str) -> Value {
    let fields = Fields::new()
        .with("key", item.key.as_str())
        .with("state", item_state(item.state))
        .maybe("step", item.step.as_deref())
        .maybe("iteration", item.iteration)
        .maybe("attempt", item.attempt)
        .time("since", item.since)
        .maybe("merged_sha", item.merged_as.as_deref())
        .list("waits_for", item.waits_for.clone())
        .maybe("conflict", item.conflict.as_ref().map(conflict));
    let Some(step) = status
        .blocked
        .iter()
        .find(|step| step.step.item.as_deref() == Some(item.key.as_str()))
    else {
        return fields.into();
    };
    fields
        .with("blocked_reason", reason(step.reason))
        .maybe("last_error", step.last_failure.as_ref().map(last_error))
        .with("resume", resume)
        .into()
}

fn item_state(state: ItemState) -> &'static str {
    match state {
        ItemState::Pending => "pending",
        ItemState::Queued => "queued",
        ItemState::Running => "running",
        ItemState::Interrupted => "interrupted",
        ItemState::Blocked => "blocked",
        ItemState::Succeeded => "succeeded",
        ItemState::Cancelled => "cancelled",
    }
}

fn conflict(conflict: &Conflict) -> Fields {
    match conflict {
        Conflict::Merge { paths } => Fields::new()
            .with("kind", "merge")
            .with("paths", paths.clone()),
        Conflict::Checks { failed } => Fields::new()
            .with("kind", "checks")
            .with("failed", failed.clone()),
    }
}

fn blocked(step: &BlockedStep) -> Value {
    Fields::new()
        .with("step", step.step.key.as_str())
        .maybe("task", step.step.item.as_deref())
        .maybe("iteration", step.step.iteration)
        .with("reason", reason(step.reason))
        .maybe("last_error", step.last_failure.as_ref().map(last_error))
        .into()
}

fn reason(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::AttemptsExhausted => "attempts_exhausted",
        BlockReason::NotRetryable => "not_retryable",
        BlockReason::IterationsExhausted => "iterations_exhausted",
    }
}

fn last_error(failure: &LastFailure) -> Fields {
    Fields::new()
        .with("attempt", failure.attempt)
        .with("code", failure.code.as_str())
        .with("message", failure.message.as_str())
        .time("resets_at", failure.resets_at)
}
