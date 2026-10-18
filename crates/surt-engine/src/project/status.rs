mod facts;
mod gates;
mod tasks;

pub(super) use gates::origin;

use std::path::PathBuf;

use super::{BlockedStep, Commit, Error, GateStatus, IntegrationBranch, Project, RevisionOrigin};
use crate::cleanup::{Kept, Left};
use crate::drive::StepLabel;
use crate::git;
use crate::platform;
use crate::store::{RootRunRow, Store};

const SUCCEEDED: &str = "succeeded";
const CANCELLED: &str = "cancelled";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusScope<'a> {
    Active,
    All,
    Feature(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunStatus {
    pub subject: String,
    pub run: u32,
    pub states: Vec<RunState>,
    pub driver: Option<Driver>,
    pub base: Option<Commit>,
    pub branch: Option<IntegrationBranch>,
    pub gates: Vec<GateView>,
    pub tasks: Vec<TaskStatus>,
    pub blocked: Vec<BlockedStep>,
    pub unattended: Vec<Unattended>,
    pub integrating: Vec<PendingIntegration>,
    pub cleanup: Option<CleanupState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    AwaitingApproval,
    Blocked,
    Interrupted,
    Succeeded,
    Cancelled,
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
    pub files: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskStatus {
    pub key: String,
    pub state: TaskState,
    pub step: Option<String>,
    pub iteration: Option<u32>,
    pub attempt: Option<u32>,
    pub since: Option<i64>,
    pub merged_as: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
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
}

impl Project {
    pub async fn status(&self, scope: StatusScope<'_>) -> Result<Vec<RunStatus>, Error> {
        let (subject, terminal) = match scope {
            StatusScope::Active => (None, false),
            StatusScope::All => (None, true),
            StatusScope::Feature(reference) => {
                (Some(self.resolve_subject(reference).await?.name), true)
            }
        };
        let store = self.reader().await?;
        let statuses = self
            .read_statuses(&store, subject.as_deref(), terminal)
            .await;
        store.close().await;
        statuses
    }

    async fn read_statuses(
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

    async fn run_status(&self, store: &Store, run: &RootRunRow) -> Result<RunStatus, Error> {
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
            .map(|row| self.blocked_step(&run.subject, row))
            .collect::<Vec<_>>();
        let waiting = !store.waiting_gates(root).await?.is_empty();
        Ok(RunStatus {
            subject: run.subject.clone(),
            run: run.ordinal(),
            states: states(&run.state, driver.is_some(), waiting, !blocked.is_empty()),
            base: facts::base(store, root).await?,
            branch: store
                .run_branch(root)
                .await?
                .map(|branch| IntegrationBranch {
                    name: git::short_branch(&branch.ref_name).to_owned(),
                    tip: branch.tip_sha,
                }),
            gates: gates::views(store, &self.layout, run).await?,
            tasks: tasks::statuses(store, root).await?,
            unattended: facts::unattended(store, root, driver.is_some()).await?,
            integrating: facts::integrating(store, root).await?,
            cleanup: facts::cleanup(store, run).await?,
            driver,
            blocked,
        })
    }
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
