use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::json;
use tokio::sync::Mutex;

use super::decide::{self, Action, Ctx, RunEnd};
use super::ending;
use super::flights::Flights;
use super::frozen::{CorruptSnapshot, Frozen};
use super::graph::{RunId, StepRunId};
use super::load::Loaded;
use super::operate::Flight;
use super::outcome::{ActivitySnapshot, DriveOutcome, Progress, ProgressSink};
use super::recovered;
use super::slots::FreeSlots;
use crate::artifacts::working_copy::{WorkingCopy, finish_pending_writes};
use crate::platform;
use crate::project::{Error, Layout, Refusal};
use crate::reconcile::Reconcile;
use crate::runtime::{AgentRuntime, StopLevel, StopSignal};
use crate::store::{StepRow, Store};
use crate::workflow::{Catalog, Definition};

const TICK: Duration = Duration::from_secs(1);
const DONE: &str = "done";
const DRIVER_STOPPED: &str = "driver.stopped";

pub(crate) struct Driving<'a> {
    pub(crate) store: &'a Store,
    pub(crate) layout: &'a Layout,
    pub(crate) root: &'a RunId,
    pub(crate) max_agents: usize,
    pub(crate) worktrees: Option<PathBuf>,
    pub(crate) actor: &'a str,
    pub(crate) catalog: &'a Catalog,
    pub(crate) stop_grace_period: Duration,
}

pub(super) struct Driver<'a, R> {
    pub(super) driving: Driving<'a>,
    pub(super) frozen: Frozen,
    pub(super) working_copy: WorkingCopy,
    pub(super) runtime: &'a R,
    progress: ProgressSink,
    stop: StopSignal,
    pub(super) agents: StopSignal,
    pub(super) cancelling: AtomicBool,
    pub(super) branches: Mutex<()>,
    pub(super) activity: std::sync::Mutex<Option<ActivitySnapshot>>,
}

enum Tick {
    Again,
    Launched(Vec<Flight>),
    Wait,
    Cancel,
    Return(DriveOutcome),
}

pub(crate) async fn drive<R: AgentRuntime>(
    driving: Driving<'_>,
    runtime: &R,
    progress: ProgressSink,
    stop: StopSignal,
) -> Result<DriveOutcome, Error> {
    let path = driving.layout.run_lock(driving.root.as_str());
    let lock =
        platform::try_lock_exclusive(&path).map_err(|error| Error::io("lock", &path, error))?;
    let Some(_lock) = lock else {
        return Err(driver_live(&driving).await?);
    };
    if let Some(outcome) = ending::ended_before(&driving, &progress).await? {
        return Ok(outcome);
    }
    let (store, root) = (driving.store, driving.root);
    let reconcile = Reconcile {
        store,
        repository: driving.layout.root(),
        root: root.as_str(),
        stop_grace_period: driving.stop_grace_period,
    };
    recover(&driving, &reconcile, &progress).await?;
    let outcome = drive_recovered(driving, &reconcile, runtime, progress, stop).await?;
    let stopped = json!({ "pid": std::process::id() });
    store
        .record_run_event(root.as_str(), DRIVER_STOPPED, &stopped)
        .await?;
    Ok(outcome)
}

async fn drive_recovered<R: AgentRuntime>(
    driving: Driving<'_>,
    reconcile: &Reconcile<'_>,
    runtime: &R,
    progress: ProgressSink,
    stop: StopSignal,
) -> Result<DriveOutcome, Error> {
    let store = driving.store;
    let frozen = Frozen::from_files(store.frozen_files(driving.root.as_str()).await?)?;
    let working_copy =
        WorkingCopy::load(store, driving.layout, driving.root.as_str(), &frozen).await?;
    let driver = Driver {
        driving,
        frozen,
        working_copy,
        runtime,
        progress,
        stop,
        agents: StopSignal::new(),
        cancelling: AtomicBool::new(false),
        branches: Mutex::new(()),
        activity: std::sync::Mutex::new(None),
    };
    if let Some(refusal) = driver.guard().await? {
        return Ok(DriveOutcome::Halted(refusal));
    }
    if let Some(refusal) = reconcile.check_integration_branch().await? {
        return Ok(DriveOutcome::Halted(refusal));
    }
    store.unblock(driver.driving.root).await?;
    let outcome = driver.run().await?;
    let ended = ending::ended_while_driving(&driver.driving, &driver.progress).await?;
    Ok(ended.unwrap_or(outcome))
}

async fn recover(
    driving: &Driving<'_>,
    reconcile: &Reconcile<'_>,
    progress: &ProgressSink,
) -> Result<(), Error> {
    let store = driving.store;
    let previous = store.driver(driving.root).await?.pid;
    let pid = std::process::id();
    store
        .record_driver(driving.root, pid, platform::identify(pid).as_ref())
        .await?;
    let found = reconcile.settle().await?;
    if !found.is_empty() {
        let driver = previous.and_then(|pid| u32::try_from(pid).ok());
        let data = recovered::recovered(driver, &found);
        let root = driving.root.as_str();
        store
            .record_run_event(root, recovered::DRIVER_RECOVERED, &data)
            .await?;
        let _ = progress.send(Progress::Recovering { driver });
        for recovered in found {
            let _ = progress.send(recovered);
        }
    }
    let subject = store.subject_of_run(driving.root.as_str()).await?;
    let working_copy = driving.layout.working_copy(&subject);
    finish_pending_writes(store, &working_copy, driving.root.as_str()).await
}

async fn driver_live(driving: &Driving<'_>) -> Result<Error, Error> {
    let record = driving.store.driver(driving.root).await?;
    Ok(Refusal::DriverLive {
        subject: record.subject,
        pid: record.pid.and_then(|pid| u32::try_from(pid).ok()),
        since: record.since,
    }
    .into())
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn report(&self, progress: Progress) {
        let _ = self.progress.send(progress);
    }

    async fn run(&self) -> Result<DriveOutcome, Error> {
        let mut flights = Flights::new();
        loop {
            let level = self.stop.level();
            self.agents.raise(level);
            if level == StopLevel::Force {
                return Ok(DriveOutcome::Interrupted);
            }
            let limit = self.driving.max_agents;
            let mut free = FreeSlots::take(self.driving.layout, limit, &flights.held())?;
            let (tick, landed) = flights.alongside(self.tick(free.indices(), limit)).await;
            let stale = !landed.is_empty();
            settle(landed)?;
            match tick? {
                Tick::Again => {}
                Tick::Wait | Tick::Return(_) if stale => {}
                Tick::Launched(launches) => {
                    for flight in launches {
                        let slot = flight.slot().and_then(|index| free.claim(index));
                        flights.push(slot, self.fly(flight));
                    }
                }
                Tick::Return(outcome) if flights.is_empty() => return Ok(outcome),
                Tick::Return(_) | Tick::Wait => {
                    drop(free);
                    self.wait(&mut flights, level).await?;
                }
                Tick::Cancel => {
                    drop(free);
                    self.cancel(&mut flights).await?;
                }
            }
        }
    }

    async fn wait<F>(&self, flights: &mut Flights<F>, seen: StopLevel) -> Result<(), Error>
    where
        F: Future<Output = Result<(), Error>>,
    {
        let busy = !flights.is_empty();
        tokio::select! {
            landed = flights.landed(), if busy => settle(landed),
            () = tokio::time::sleep(TICK) => Ok(()),
            () = self.stop.raised_above(seen) => Ok(()),
        }
    }

    async fn cancel<F>(&self, flights: &mut Flights<F>) -> Result<(), Error>
    where
        F: Future<Output = Result<(), Error>>,
    {
        self.cancelling.store(true, Ordering::SeqCst);
        self.agents.raise(StopLevel::Graceful);
        while !flights.is_empty() {
            settle(flights.landed().await)?;
        }
        Ok(self.driving.store.cancel_run(self.driving.root).await?)
    }

    async fn tick(&self, free_slots: Vec<usize>, limit: usize) -> Result<Tick, Error> {
        let rows = self.driving.store.run_rows(self.driving.root).await?;
        self.read_stop_request(rows.stop_requested);
        let lists = self.item_lists(&rows).await?;
        let loaded = Loaded::new(&self.frozen, rows, &lists)?;
        let ctx = Ctx {
            limits: self.frozen.limits,
            free_slots,
            stop: stop_level(self.stop.level()),
        };
        let mut launches = Vec::new();
        let decision = decide::decision(&loaded.graph, &ctx);
        let actions = decision.actions;
        if let Some(refusal) = self.guard_readers(&actions).await? {
            return Ok(Tick::Return(DriveOutcome::Halted(refusal)));
        }
        for action in actions {
            match action {
                Action::Launch { step, slot } => {
                    launches.push(Flight::Agent(Box::new(
                        self.launch(&loaded, &step, slot).await?,
                    )));
                }
                Action::RunOperation { step, op } => {
                    launches.push(Flight::Operation(Box::new(
                        self.start_operation(&loaded, &step, &op).await?,
                    )));
                }
                Action::FinishRun {
                    end: RunEnd::Cancelled,
                    ..
                } => return Ok(Tick::Cancel),
                Action::Wait => {
                    let in_use = limit.saturating_sub(ctx.free_slots.len());
                    self.observe(&loaded, decision.queued, in_use, limit);
                    return Ok(Tick::Wait);
                }
                Action::Return(outcome) => {
                    return Ok(Tick::Return(self.public(&loaded, outcome)?));
                }
                action => self.act(&loaded, action).await?,
            }
        }
        if launches.is_empty() {
            Ok(Tick::Again)
        } else {
            Ok(Tick::Launched(launches))
        }
    }

    fn read_stop_request(&self, requested: bool) {
        if requested && self.stop.level() == StopLevel::None {
            self.stop.graceful();
            self.report(Progress::StopRequested);
        }
    }

    async fn act(&self, loaded: &Loaded, action: Action) -> Result<(), Error> {
        let store = self.driving.store;
        match action {
            Action::StartIteration { loop_ } => self.start_iteration(loaded, &loop_).await?,
            Action::ExpandItems { step } => self.expand(loaded, &step).await?,
            Action::OpenGate { step } => store.open_step(&step.0).await?,
            Action::Skip { step } => store.skip_step(&step.0).await?,
            Action::Block { step, reason } => store.block_step(&step.0, reason).await?,
            Action::FinishStep { step } => store.finish_step(&step.0, DONE).await?,
            Action::FinishRun { run, end } => store.finish_run(&run.0, end).await?,
            Action::Launch { .. }
            | Action::RunOperation { .. }
            | Action::Wait
            | Action::Return(_) => {}
        }
        Ok(())
    }

    pub(super) fn definition(
        &self,
        loaded: &Loaded,
        row: &StepRow,
    ) -> Result<&Definition, CorruptSnapshot> {
        self.frozen.definition(loaded.definition_of(&row.run))
    }
}

fn settle(landed: Vec<Result<(), Error>>) -> Result<(), Error> {
    landed.into_iter().collect()
}

pub(super) fn known<'a>(
    loaded: &'a Loaded,
    step: &StepRunId,
) -> Result<&'a StepRow, CorruptSnapshot> {
    loaded
        .step(step)
        .ok_or_else(|| CorruptSnapshot::new(&step.0, "no such step run"))
}

fn stop_level(level: StopLevel) -> decide::StopLevel {
    match level {
        StopLevel::None => decide::StopLevel::None,
        StopLevel::Graceful => decide::StopLevel::Graceful,
        StopLevel::Force => decide::StopLevel::Force,
    }
}
