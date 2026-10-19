use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Conflict, ItemState, ItemStatus};
use crate::artifacts::listed_items;
use crate::drive::Frozen;
use crate::project::Error;
use crate::store::{ItemRunRow, RootRunRow, Store};
use crate::workflow::StepSpec;

const CONFLICT_ARTIFACT: &str = "conflict";
const AGENT: &str = "agent";
const PENDING: &str = "pending";
const RUNNING: &str = "running";
const PASSED: &str = "passed";

#[derive(Clone, Copy)]
pub(super) struct Readiness {
    pub(super) driven: bool,
    pub(super) slots_full: bool,
    pub(super) paused: bool,
}

impl Readiness {
    fn waits_to_launch(self) -> bool {
        self.driven && (self.slots_full || self.paused)
    }
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
) -> Result<Vec<ItemStatus>, Error> {
    let rows = store.item_runs(&run.id).await?;
    let needs = needs(store, run, frozen).await?;
    let mut statuses = Vec::new();
    for row in &rows {
        let waits_for = unmet(&rows, needs.get(&row.key));
        let (state, step) = standing(row, &waits_for, readiness);
        let conflict = match state {
            ItemState::Succeeded | ItemState::Cancelled => None,
            _ => conflict(store, &row.run).await?,
        };
        statuses.push(ItemStatus {
            key: row.key.clone(),
            state,
            step: step.map(str::to_owned),
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

pub(super) fn item_step(frozen: &Frozen, run: &RootRunRow) -> Result<Option<String>, Error> {
    Ok(frozen
        .definition(&run.definition)?
        .steps
        .iter()
        .find(|step| matches!(step.spec, StepSpec::PerItem(_)))
        .map(|step| step.key.clone()))
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

fn unmet(rows: &[ItemRunRow], needs: Option<&Vec<String>>) -> Vec<String> {
    needs
        .into_iter()
        .flatten()
        .filter(|need| {
            !rows
                .iter()
                .any(|row| &row.key == *need && state_of_run(row) == Some(ItemState::Succeeded))
        })
        .cloned()
        .collect()
}

fn standing<'a>(
    row: &'a ItemRunRow,
    waits_for: &[String],
    readiness: Readiness,
) -> (ItemState, Option<&'a str>) {
    let state = state(row, waits_for, readiness);
    let shows_step = state != ItemState::Queued
        || row.step_state.as_deref() == Some(PENDING)
        || (readiness.paused && relaunches_agent(row));
    (state, row.step_key.as_deref().filter(|_| shows_step))
}

fn state(row: &ItemRunRow, waits_for: &[String], readiness: Readiness) -> ItemState {
    if let Some(ended) = state_of_run(row) {
        return ended;
    }
    match (row.step_state.as_deref(), row.execution_state.as_deref()) {
        (Some("blocked"), _) => ItemState::Blocked,
        (Some("running"), Some("running")) => ItemState::Running,
        (Some("running"), Some("queued")) => ItemState::Queued,
        _ if !waits_for.is_empty() => ItemState::Pending,
        _ if readiness.waits_to_launch() && next_is_agent(row) => ItemState::Queued,
        (None | Some(PENDING), _) => ItemState::Pending,
        _ => ItemState::Interrupted,
    }
}

fn state_of_run(row: &ItemRunRow) -> Option<ItemState> {
    match row.run_state.as_str() {
        "succeeded" => Some(ItemState::Succeeded),
        "cancelled" => Some(ItemState::Cancelled),
        _ => None,
    }
}

fn relaunches_agent(row: &ItemRunRow) -> bool {
    row.step_kind.as_deref() == Some(AGENT) && row.step_state.as_deref() == Some(RUNNING)
}

fn next_is_agent(row: &ItemRunRow) -> bool {
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

#[cfg(test)]
mod tests;
