use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Conflict, TaskState, TaskStatus};
use crate::artifacts::listed_items;
use crate::drive::Frozen;
use crate::project::Error;
use crate::store::{RootRunRow, Store, TaskRow};
use crate::workflow::StepSpec;

const CONFLICT_ARTIFACT: &str = "conflict";
const AGENT: &str = "agent";
const PENDING: &str = "pending";
const PASSED: &str = "passed";

#[derive(Clone, Copy)]
pub(super) struct Readiness {
    pub(super) driven: bool,
    pub(super) slots_full: bool,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ConflictData {
    Merge { paths: Vec<String> },
    Checks { checks: Vec<CheckData> },
}

#[derive(Deserialize)]
struct CheckData {
    name: String,
    status: String,
}

pub(super) async fn statuses(
    store: &Store,
    run: &RootRunRow,
    frozen: &Frozen,
    readiness: Readiness,
) -> Result<Vec<TaskStatus>, Error> {
    let rows = store.tasks(&run.id).await?;
    let needs = needs(store, run, frozen).await?;
    let mut statuses = Vec::new();
    for row in &rows {
        let waits_for = unmet(&rows, needs.get(&row.key));
        let state = state(row, &waits_for, readiness);
        let conflict = match state {
            TaskState::Succeeded | TaskState::Cancelled => None,
            _ => conflict(store, &row.run).await?,
        };
        let shows_step = state != TaskState::Queued || row.step_state.as_deref() == Some(PENDING);
        statuses.push(TaskStatus {
            key: row.key.clone(),
            state,
            step: row.step_key.clone().filter(|_| shows_step),
            iteration: row.iteration.and_then(|value| u32::try_from(value).ok()),
            attempt: row.attempt.and_then(|value| u32::try_from(value).ok()),
            since: row.started_at,
            merged_as: row.merged_as.clone(),
            waits_for,
            conflict,
        });
    }
    Ok(statuses)
}

async fn needs(
    store: &Store,
    run: &RootRunRow,
    frozen: &Frozen,
) -> Result<BTreeMap<String, Vec<String>>, Error> {
    let mut needs = BTreeMap::new();
    for step in &frozen.definition(&run.definition)?.steps {
        let StepSpec::PerItem(spec) = &step.spec else {
            continue;
        };
        let Some(revision) = store.expanded_revision(&run.id, &spec.items).await? else {
            continue;
        };
        for item in listed_items(&revision.files).unwrap_or_default() {
            needs.insert(item.key, item.needs);
        }
    }
    Ok(needs)
}

fn unmet(rows: &[TaskRow], needs: Option<&Vec<String>>) -> Vec<String> {
    needs
        .into_iter()
        .flatten()
        .filter(|need| {
            !rows
                .iter()
                .any(|row| &row.key == *need && state_of_run(row) == Some(TaskState::Succeeded))
        })
        .cloned()
        .collect()
}

fn state(row: &TaskRow, waits_for: &[String], readiness: Readiness) -> TaskState {
    if let Some(ended) = state_of_run(row) {
        return ended;
    }
    match (row.step_state.as_deref(), row.execution_state.as_deref()) {
        (Some("blocked"), _) => TaskState::Blocked,
        (Some("running"), Some("running")) => TaskState::Running,
        (Some("running"), Some("queued")) => TaskState::Queued,
        _ if !waits_for.is_empty() => TaskState::Pending,
        _ if readiness.driven && readiness.slots_full && next_is_agent(row) => TaskState::Queued,
        (None | Some(PENDING), _) => TaskState::Pending,
        _ => TaskState::Interrupted,
    }
}

fn state_of_run(row: &TaskRow) -> Option<TaskState> {
    match row.run_state.as_str() {
        "succeeded" => Some(TaskState::Succeeded),
        "cancelled" => Some(TaskState::Cancelled),
        _ => None,
    }
}

fn next_is_agent(row: &TaskRow) -> bool {
    row.step_state.as_deref() != Some(PENDING) || row.step_kind.as_deref() == Some(AGENT)
}

async fn conflict(store: &Store, run: &str) -> Result<Option<Conflict>, Error> {
    let Some(id) = store.revision_id(run, CONFLICT_ARTIFACT, None).await? else {
        return Ok(None);
    };
    let revision = store.revision(&id).await?;
    Ok(revision
        .files
        .first()
        .and_then(|(_, bytes)| serde_json::from_slice::<ConflictData>(bytes).ok())
        .map(|data| match data {
            ConflictData::Merge { paths } => Conflict::Merge { paths },
            ConflictData::Checks { checks } => Conflict::Checks {
                failed: checks
                    .into_iter()
                    .filter(|check| check.status != PASSED)
                    .map(|check| check.name)
                    .collect(),
            },
        }))
}
