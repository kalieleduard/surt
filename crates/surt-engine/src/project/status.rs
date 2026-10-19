mod facts;
mod gates;
mod items;

pub(in crate::project) use facts::{base as recorded_base, merge as recorded_merge};
pub(super) use gates::origin;

use std::path::PathBuf;

use super::{
    BlockedStep, Commit, Error, GateStatus, IntegrationBranch, MergeIntoBase, Project,
    RevisionOrigin, RunSelector, RunState,
};
use crate::cleanup::{Kept, Left};
use crate::drive::{BaseMerge, Frozen, StepLabel};
use crate::git;
use crate::platform;
use crate::runtime::FailureClass;
use crate::store::{LastAgentEnd, RootRunRow, Store};

const SUCCEEDED: &str = "succeeded";
const CANCELLED: &str = "cancelled";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusScope<'a> {
    Active,
    All,
    Subject(&'a str, RunSelector),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunStatus {
    pub subject: String,
    pub run: u32,
    pub runs: u32,
    pub states: Vec<RunState>,
    pub ended_at: Option<i64>,
    pub driver: Option<Driver>,
    pub base: Option<Commit>,
    pub branch: Option<IntegrationBranch>,
    pub merge_into_base: MergeIntoBase,
    pub merge: Option<BaseMerge>,
    pub slots: AgentSlots,
    pub paused: Option<LaunchPause>,
    pub gates: Vec<GateView>,
    pub item_step: Option<String>,
    pub items: Vec<ItemStatus>,
    pub blocked: Vec<BlockedStep>,
    pub unattended: Vec<Unattended>,
    pub integrating: Vec<PendingIntegration>,
    pub cleanup: Option<CleanupState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AgentSlots {
    pub in_use: u32,
    pub total: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchPause {
    pub code: String,
    pub resets_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Driver {
    pub pid: u32,
    pub since: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateView {
    pub step: String,
    pub artifact: String,
    pub status: GateStatus,
    pub revision: Option<RevisionSummary>,
    pub decided_by: Option<String>,
    pub decided_at: Option<i64>,
    pub working_copy: PathBuf,
    pub edited: Vec<String>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionSummary {
    pub number: u32,
    pub origin: RevisionOrigin,
    pub sha256: String,
    pub files: Vec<String>,
    pub item_list: Option<ItemListSummary>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemListSummary {
    pub items: usize,
    pub item_files: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStatus {
    pub key: String,
    pub state: ItemState,
    pub step: Option<String>,
    pub iteration: Option<u32>,
    pub attempt: Option<u32>,
    pub since: Option<i64>,
    pub merged_as: Option<String>,
    pub waits_for: Vec<String>,
    pub conflict: Option<Conflict>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conflict {
    Merge { paths: Vec<String> },
    Checks { failed: Vec<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Pending,
    Queued,
    Running,
    Interrupted,
    Blocked,
    Succeeded,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unattended {
    pub step: StepLabel,
    pub pid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingIntegration {
    pub step: StepLabel,
    pub branch: String,
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupState {
    Pending { left: Vec<Left>, kept: Vec<Kept> },
    Finished { kept: Vec<Kept> },
    Kept,
}

impl Project {
    pub async fn status(&self, scope: StatusScope<'_>) -> Result<Vec<RunStatus>, Error> {
        let selected = match scope {
            StatusScope::Subject(reference, run) => {
                Some((self.resolve_subject(reference).await?.name, run))
            }
            _ => None,
        };
        let store = self.reader().await?;
        let statuses = match &selected {
            Some((subject, run)) => self.selected_status(&store, subject, *run).await,
            None => {
                self.read_statuses(&store, None, scope == StatusScope::All)
                    .await
            }
        };
        store.close().await;
        statuses
    }

    async fn selected_status(
        &self,
        store: &Store,
        subject: &str,
        run: RunSelector,
    ) -> Result<Vec<RunStatus>, Error> {
        let run = self.select_run(store, subject, run).await?;
        Ok(vec![self.run_status(store, &run).await?])
    }

    pub(super) async fn read_statuses(
        &self,
        store: &Store,
        subject: Option<&str>,
        terminal: bool,
    ) -> Result<Vec<RunStatus>, Error> {
        let mut statuses = Vec::new();
        for run in store.runs_to_show(subject, terminal).await? {
            statuses.push(self.run_status(store, &run).await?);
        }
        Ok(statuses)
    }

    pub(super) async fn run_status(
        &self,
        store: &Store,
        run: &RootRunRow,
    ) -> Result<RunStatus, Error> {
        let root = run.id.as_str();
        let driver = run
            .driver()
            .filter(platform::is_alive)
            .map(|identity| Driver {
                pid: identity.pid,
                since: run.driver_since,
            });
        let blocked = store
            .blocked_steps(root)
            .await?
            .into_iter()
            .map(BlockedStep::from)
            .collect::<Vec<_>>();
        let waiting = !store.waiting_gates(root).await?.is_empty();
        let frozen = Frozen::from_files(store.frozen_files(root).await?)?;
        let slots = AgentSlots {
            in_use: slots_in_use(store).await?,
            total: self.config.max_concurrent_agents.get(),
        };
        let paused = match driver {
            Some(_) => store.last_agent_end(root).await?.and_then(pause),
            None => None,
        };
        let readiness = items::Readiness {
            driven: driver.is_some(),
            slots_full: slots.in_use >= slots.total,
            paused: paused.is_some(),
        };
        Ok(RunStatus {
            subject: run.subject.clone(),
            run: run.ordinal(),
            runs: run.runs(),
            ended_at: run.finished_at,
            states: states(&run.state, driver.is_some(), waiting, !blocked.is_empty()),
            base: facts::base(store, root).await?,
            branch: store
                .run_branch(root)
                .await?
                .map(|branch| IntegrationBranch {
                    name: git::short_branch(&branch.ref_name).to_owned(),
                    tip: branch.tip_sha,
                }),
            merge_into_base: frozen.merge_into_base,
            merge: facts::merge(store, root).await?,
            slots,
            paused,
            gates: gates::views(store, &self.layout, run).await?,
            item_step: items::item_step(&frozen, run)?,
            items: items::statuses(store, run, &frozen, readiness).await?,
            unattended: facts::unattended(store, root, driver.is_some()).await?,
            integrating: facts::integrating(store, root).await?,
            cleanup: facts::cleanup(store, run, frozen.on_success).await?,
            driver,
            blocked,
        })
    }
}

fn pause(end: LastAgentEnd) -> Option<LaunchPause> {
    if end.failure_class() != Some(FailureClass::Transient) || end.started_after {
        return None;
    }
    Some(LaunchPause {
        code: end.error_code.unwrap_or_default(),
        resets_at: end.resets_at.filter(|reset| *reset > end.finished_at),
    })
}

async fn slots_in_use(store: &Store) -> Result<u32, Error> {
    Ok(store
        .slot_holders()
        .await?
        .iter()
        .filter(|holders| {
            holders
                .driver()
                .is_some_and(|driver| platform::is_alive(&driver))
        })
        .map(|holders| holders.agents())
        .sum())
}

fn states(stored: &str, driven: bool, waiting: bool, blocked: bool) -> Vec<RunState> {
    match stored {
        SUCCEEDED => return vec![RunState::Succeeded],
        CANCELLED => return vec![RunState::Cancelled],
        _ if driven => return vec![RunState::Running],
        _ => {}
    }
    let mut states = Vec::new();
    if waiting {
        states.push(RunState::AwaitingApproval);
    }
    if blocked {
        states.push(RunState::Blocked);
    }
    if states.is_empty() {
        states.push(RunState::Interrupted);
    }
    states
}
