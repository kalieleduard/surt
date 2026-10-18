use std::time::Duration;

use super::decide::{self, Action, Ctx};
use super::frozen::{CorruptSnapshot, Frozen};
use super::graph::StepRunId;
use super::load::{Loaded, label};
use super::outcome::{DriveOutcome, Gate, Progress, ProgressSink};
use crate::platform;
use crate::project::{Error, Layout, Refusal};
use crate::runtime::{AgentRuntime, StopLevel, StopSignal};
use crate::store::{StepRow, Store};
use crate::workflow::{Definition, StepSpec};
use crate::working_copy::WorkingCopy;

const TICK: Duration = Duration::from_secs(1);
const DONE: &str = "done";

pub(crate) struct Driving<'a> {
    pub(crate) store: &'a Store,
    pub(crate) layout: &'a Layout,
    pub(crate) root: &'a str,
    pub(crate) max_agents: usize,
    pub(crate) actor: &'a str,
}

pub(super) struct Driver<'a, R> {
    pub(super) driving: Driving<'a>,
    pub(super) frozen: Frozen,
    pub(super) working_copy: WorkingCopy,
    pub(super) runtime: &'a R,
    progress: ProgressSink,
    pub(super) stop: StopSignal,
}

#[derive(Debug, thiserror::Error)]
#[error("step {step} needs a capability this surt does not have yet")]
struct Unsupported {
    step: String,
}

pub(crate) async fn drive<R: AgentRuntime>(
    driving: Driving<'_>,
    runtime: &R,
    progress: ProgressSink,
    stop: StopSignal,
) -> Result<DriveOutcome, Error> {
    let path = driving.layout.run_lock(driving.root);
    let lock =
        platform::try_lock_exclusive(&path).map_err(|error| Error::io("lock", &path, error))?;
    let Some(_lock) = lock else {
        return Err(driver_live(&driving).await?);
    };
    driving
        .store
        .record_driver(driving.root, std::process::id())
        .await?;
    driving.store.interrupt_stale(driving.root).await?;
    let frozen = Frozen::from_files(driving.store.frozen_files(driving.root).await?)?;
    let working_copy =
        WorkingCopy::load(driving.store, driving.layout, driving.root, &frozen).await?;
    let driver = Driver {
        driving,
        frozen,
        working_copy,
        runtime,
        progress,
        stop,
    };
    if let Some(refusal) = driver.guard().await? {
        return Ok(DriveOutcome::Halted(refusal));
    }
    loop {
        if let Some(outcome) = driver.tick().await? {
            return Ok(outcome);
        }
    }
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

    async fn tick(&self) -> Result<Option<DriveOutcome>, Error> {
        let rows = self.driving.store.run_rows(self.driving.root).await?;
        let definition = self.frozen.definition(&rows.definition)?;
        let loaded = Loaded::new(definition, rows);
        let ctx = Ctx {
            limits: self.frozen.limits,
            free_slots: self.driving.max_agents,
            stop: stop_level(self.stop.level()),
        };
        for action in decide::decide(&loaded.graph, &ctx) {
            if let Some(outcome) = self.act(&loaded, definition, action).await? {
                return Ok(Some(outcome));
            }
        }
        Ok(None)
    }

    async fn act(
        &self,
        loaded: &Loaded,
        definition: &Definition,
        action: Action,
    ) -> Result<Option<DriveOutcome>, Error> {
        let store = self.driving.store;
        match action {
            Action::StartIteration { loop_ } => {
                self.start_iteration(loaded, definition, &loop_).await?;
            }
            Action::Launch { step } => {
                if let Some(refusal) = self.launch(loaded, definition, &step).await? {
                    return Ok(Some(DriveOutcome::Halted(refusal)));
                }
            }
            Action::OpenGate { step } => store.open_step(&step.0).await?,
            Action::Skip { step } => store.skip_step(&step.0).await?,
            Action::Block { step, reason } => store.block_step(&step.0, reason).await?,
            Action::FinishStep { step } => store.finish_step(&step.0, DONE).await?,
            Action::FinishRun { run, end } => store.finish_run(&run.0, end).await?,
            Action::ExpandItems { step } | Action::RunOperation { step, .. } => {
                return Err(Error::internal(Unsupported {
                    step: known(loaded, &step)?.key.clone(),
                }));
            }
            Action::Wait => tokio::time::sleep(TICK).await,
            Action::Return(outcome) => return Ok(Some(public(loaded, definition, outcome))),
        }
        Ok(None)
    }

    async fn start_iteration(
        &self,
        loaded: &Loaded,
        definition: &Definition,
        loop_: &StepRunId,
    ) -> Result<(), Error> {
        let row = known(loaded, loop_)?;
        let Some(StepSpec::Loop(spec)) = definition.step(&row.key).map(|step| &step.spec) else {
            return Err(CorruptSnapshot::new(&row.key, "not a loop").into());
        };
        let body: Vec<(&str, &str)> = spec
            .body
            .iter()
            .filter_map(|key| definition.step(key))
            .map(|step| (step.key.as_str(), step.spec.kind().as_str()))
            .collect();
        Ok(self.driving.store.start_iteration(&row.id, &body).await?)
    }
}

pub(super) fn known<'a>(
    loaded: &'a Loaded,
    step: &StepRunId,
) -> Result<&'a StepRow, CorruptSnapshot> {
    loaded
        .step(step)
        .ok_or_else(|| CorruptSnapshot::new(&step.0, "no such step run"))
}

fn public(loaded: &Loaded, definition: &Definition, outcome: decide::DriveOutcome) -> DriveOutcome {
    let rows = |steps: &[StepRunId]| {
        steps
            .iter()
            .filter_map(|step| loaded.step(step))
            .collect::<Vec<_>>()
    };
    match outcome {
        decide::DriveOutcome::AtGate { gates } => DriveOutcome::AtGate {
            gates: rows(&gates)
                .into_iter()
                .map(|row| Gate {
                    step: label(row),
                    artifact: gate_artifact(definition, &row.key),
                })
                .collect(),
        },
        decide::DriveOutcome::Blocked { steps } => DriveOutcome::Blocked {
            steps: rows(&steps).into_iter().map(label).collect(),
        },
        decide::DriveOutcome::Succeeded => DriveOutcome::Succeeded,
        decide::DriveOutcome::Cancelled => DriveOutcome::Cancelled,
        decide::DriveOutcome::Interrupted => DriveOutcome::Interrupted,
    }
}

fn gate_artifact(definition: &Definition, key: &str) -> String {
    match definition.step(key).map(|step| &step.spec) {
        Some(StepSpec::Approval(gate)) => gate.artifact.clone(),
        _ => String::new(),
    }
}

fn stop_level(level: StopLevel) -> decide::StopLevel {
    match level {
        StopLevel::None => decide::StopLevel::None,
        StopLevel::Graceful => decide::StopLevel::Graceful,
        StopLevel::Force => decide::StopLevel::Force,
    }
}
