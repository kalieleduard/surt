use std::fs::{self, OpenOptions};
use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::json;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use uuid::Uuid;

use super::driver::{Driver, known};
use super::failure::OperationError;
use super::frozen::{CorruptSnapshot, FrozenAgent};
use super::graph::{StepRunId, StepRunState};
use super::inputs::scoped;
use super::load::{Loaded, label};
use super::outcome::Progress;
use super::pausing::{AgentEnd, unix_now};
use super::record::WorkingCopy;
use crate::artifacts::{AcceptedDelivery, DeliveryContract};
use crate::context::{Assembly, ContextInput, PreviousIteration};
use crate::platform;
use crate::project::Error;
use crate::runtime::{
    AgentRuntime, FailureClass, Rejection, RuntimeEvent, RuntimeRequest, STDERR_LOG, STREAM_LOG,
    StepAttempt, StopLevel, Verdict, accept,
};
use crate::store::{
    ExecutionEnd, ExecutionKind, ExecutionStart, RunBranch, StartedExecution, StepRow,
};
use crate::workflow::{DeliveryCheck, StepSpec};

const DELIVERY_DIR: &str = "delivery";
const LAUNCH_DIR: &str = "launch";
const LOG_FILES: [&str; 2] = [STREAM_LOG, STDERR_LOG];
const SKIPPED: &str = "SKIPPED";
const FIRST_ITERATION: u32 = 1;
const RUNTIME_VERSION: &str = "runtime_version";

pub(super) struct Launch {
    pub(super) slot: usize,
    pub(super) row: StepRow,
    pub(super) started: StartedExecution,
    launched_at: i64,
    request: RuntimeRequest,
    contract: DeliveryContract,
    pub(super) working_copy: Option<WorkingCopy>,
    pub(super) worktree: Option<RunBranch>,
    pub(super) checks: Vec<(String, DeliveryCheck)>,
}

enum Ending {
    Accepted(AcceptedDelivery),
    Ended(ExecutionEnd),
    Cancelled,
}

impl Ending {
    fn agent_end(&self, launched_at: i64) -> AgentEnd<'_> {
        match self {
            Ending::Ended(ExecutionEnd::Failed {
                class: FailureClass::Transient,
                code,
                resets_at,
                ..
            }) => AgentEnd::Transient {
                code,
                resets_at: *resets_at,
                launched_at,
            },
            _ => AgentEnd::Other,
        }
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn launch(
        &self,
        loaded: &Loaded,
        step: &StepRunId,
        slot: usize,
    ) -> Result<Launch, Error> {
        let row = known(loaded, step)?;
        let definition = self.definition(loaded, row)?;
        let Some(StepSpec::Agent(spec)) = definition.step(&row.key).map(|step| &step.spec) else {
            return Err(CorruptSnapshot::new(&row.key, "not an agent step").into());
        };
        let agent = self.frozen.agent(&spec.agent)?;
        let contract = agent.delivery_contract()?;
        let start = ExecutionStart {
            step: &row.id,
            kind: ExecutionKind::Agent {
                snapshot: &agent.snapshot,
                slot,
                runtime: agent.contract.runtime(),
            },
            inputs: self.input_requests(
                loaded,
                row,
                agent.contract.inputs.keys().map(|input| {
                    let artifact = spec.inputs.get(input).unwrap_or(input);
                    (input.as_str(), artifact.as_str())
                }),
            )?,
        };
        let started = self.driving.store.start_execution(start).await?;
        let worktree = self
            .driving
            .store
            .run_branch(&row.run)
            .await?
            .filter(|branch| branch.worktree_path.is_some());
        let request = self
            .prepare(loaded, row, agent, &started, worktree.as_ref())
            .await?;
        self.report(Progress::StepStarted {
            step: label(row),
            attempt: started.attempt,
        });
        Ok(Launch {
            slot,
            row: row.clone(),
            started,
            launched_at: unix_now(),
            request,
            contract,
            working_copy: self.working_copy(loaded, row, definition),
            worktree,
            checks: agent.contract.delivery_checks(),
        })
    }

    pub(super) async fn carry(&self, launch: Launch) -> Result<(), Error> {
        let ending = match self.unprepared(&launch).await? {
            Some(end) => Ending::Ended(end),
            None => self.execute(&launch).await?,
        };
        self.learn(&ending.agent_end(launch.launched_at));
        match ending {
            Ending::Accepted(delivery) => match failed(self.prove(&launch, &delivery).await)? {
                Some(end) => self.end(&launch.row, &launch.started, end).await,
                None => self.publish(&launch, delivery).await,
            },
            Ending::Ended(end) => self.end(&launch.row, &launch.started, end).await,
            Ending::Cancelled => Ok(()),
        }
    }

    async fn unprepared(&self, launch: &Launch) -> Result<Option<ExecutionEnd>, Error> {
        let Some(branch) = &launch.worktree else {
            return Ok(None);
        };
        failed(
            self.prepare_worktree(&launch.row, &launch.started, branch)
                .await,
        )
    }

    async fn prepare(
        &self,
        loaded: &Loaded,
        row: &StepRow,
        agent: &FrozenAgent,
        started: &StartedExecution,
        worktree: Option<&RunBranch>,
    ) -> Result<RuntimeRequest, Error> {
        let layout = self.driving.layout;
        let execution_dir = layout.execution(&started.id);
        let delivery_dir = execution_dir.join(DELIVERY_DIR);
        let log_dir = layout.execution_logs(&loaded.rows.subject, &started.id);
        fs::create_dir_all(&delivery_dir)
            .map_err(|error| Error::io("create", &delivery_dir, error))?;
        create_logs(&log_dir).map_err(|error| Error::io("create", &log_dir, error))?;
        let mut revisions = Vec::new();
        for (name, revision) in &started.inputs {
            let revision = self.driving.store.revision(revision).await?;
            revisions.push((name, scoped(name, row, revision)?));
        }
        let assembly = Assembly {
            execution_dir: &execution_dir,
            delivery_dir: &delivery_dir,
            previous: previous_iteration(loaded, row),
            inputs: revisions
                .iter()
                .map(|(name, revision)| ContextInput { name, revision })
                .collect(),
        };
        let context = assembly
            .write()
            .map_err(|error| Error::io("write the context in", &execution_dir, error))?;
        Ok(RuntimeRequest {
            step: StepAttempt::new(
                &runtime_key(row),
                row.iteration.unwrap_or(FIRST_ITERATION),
                started.attempt,
            ),
            session_id: Uuid::now_v7(),
            model: agent.contract.model().to_owned(),
            delivery_dir,
            log_dir,
            context,
            launch: agent.launch(
                worktree
                    .and_then(|branch| branch.worktree_path.clone())
                    .unwrap_or_else(|| layout.root().to_owned()),
                execution_dir.join(LAUNCH_DIR),
            )?,
        })
    }

    async fn execute(&self, launch: &Launch) -> Result<Ending, Error> {
        let (events, received) = mpsc::unbounded_channel();
        let (result, recorded) = tokio::join!(
            self.runtime
                .execute(launch.request.clone(), events, self.agents.clone()),
            self.record_identity(&launch.started.id, received),
        );
        recorded?;
        if self.cancelling.load(Ordering::SeqCst) {
            return Ok(Ending::Cancelled);
        }
        if self.agents.level() != StopLevel::None {
            return Ok(Ending::Ended(ExecutionEnd::Interrupted));
        }
        let completion = match result {
            Ok(completion) => completion,
            Err(error) => {
                return Ok(Ending::Ended(ExecutionEnd::Failed {
                    class: error.class,
                    code: error.code,
                    message: error.message,
                    resets_at: error.resets_at,
                }));
            }
        };
        Ok(
            match accept(&launch.request, &completion, &launch.contract) {
                Verdict::Accepted(delivery) => Ending::Accepted(delivery),
                Verdict::Interrupted => Ending::Ended(ExecutionEnd::Interrupted),
                Verdict::Rejected(rejection) => Ending::Ended(ExecutionEnd::Failed {
                    class: rejection.class(),
                    code: rejection_code(&rejection).to_owned(),
                    message: rejection.to_string(),
                    resets_at: None,
                }),
            },
        )
    }

    async fn record_identity(
        &self,
        execution: &str,
        mut events: UnboundedReceiver<RuntimeEvent>,
    ) -> Result<(), Error> {
        while let Some(event) = events.recv().await {
            let store = self.driving.store;
            match event {
                RuntimeEvent::Identified { version } => {
                    let fields = json!({ RUNTIME_VERSION: version });
                    store.record_runtime_info(execution, &fields).await?;
                }
                RuntimeEvent::Reported(report) => {
                    store
                        .record_runtime_info(execution, &report.fields())
                        .await?;
                }
                RuntimeEvent::Launched { pid: Some(pid) } => {
                    if let Some(identity) = platform::identify(pid) {
                        store.record_process(execution, &identity).await?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn failed(result: Result<(), OperationError>) -> Result<Option<ExecutionEnd>, Error> {
    match result {
        Ok(()) => Ok(None),
        Err(OperationError::Failed(failure)) => Ok(Some(failure.into_end())),
        Err(OperationError::Fatal(error)) => Err(error),
    }
}

fn runtime_key(row: &StepRow) -> String {
    match &row.item {
        Some(item) => format!("{item}/{}", row.key),
        None => row.key.clone(),
    }
}

fn previous_iteration(loaded: &Loaded, row: &StepRow) -> Option<PreviousIteration> {
    let previous = loaded.previous_iteration(row);
    let number = previous.first()?.iteration?;
    let ended = previous
        .into_iter()
        .map(|step| {
            let ending = match &step.state {
                StepRunState::Done(outcome) => outcome.0.clone(),
                _ => SKIPPED.to_owned(),
            };
            (step.key.clone(), ending)
        })
        .collect();
    Some(PreviousIteration { number, ended })
}

fn rejection_code(rejection: &Rejection) -> &'static str {
    match rejection {
        Rejection::Delivery(_) => "invalid_delivery",
        Rejection::ModelMismatch { .. } => "model_mismatch",
        Rejection::ExitCode(_) => "exit_code",
        _ => "invalid_result",
    }
}

fn create_logs(log_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(log_dir)?;
    for name in LOG_FILES {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
