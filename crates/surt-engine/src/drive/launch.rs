use std::sync::atomic::Ordering;

use serde_json::json;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use super::driver::{Driver, known};
use super::failure::OperationError;
use super::frozen::CorruptSnapshot;
use super::graph::StepRunId;
use super::load::{Loaded, label};
use super::outcome::Progress;
use super::pausing::{AgentEnd, unix_now};
use super::record::WorkingCopy;
use crate::artifacts::{AcceptedDelivery, DeliveryContract};
use crate::platform;
use crate::project::Error;
use crate::runtime::{
    AgentRuntime, FailureClass, Rejection, RuntimeEvent, RuntimeRequest, StopLevel, Verdict, accept,
};
use crate::store::{
    ExecutionEnd, ExecutionKind, ExecutionStart, RunBranch, StartedExecution, StepRow,
};
use crate::workflow::{DeliveryCheck, StepSpec};

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
            continued: request.continuation.is_some(),
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

    async fn execute(&self, launch: &Launch) -> Result<Ending, Error> {
        let (events, received) = mpsc::unbounded_channel();
        let (result, recorded) = tokio::join!(
            self.runtime
                .execute(launch.request.clone(), events, self.agents.clone()),
            self.record_identity(&launch.row, &launch.started.id, received),
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
        row: &StepRow,
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
                    self.report_restart(row, report.conversation.as_ref());
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

fn rejection_code(rejection: &Rejection) -> &'static str {
    match rejection {
        Rejection::Delivery(_) => "invalid_delivery",
        Rejection::ModelMismatch { .. } => "model_mismatch",
        Rejection::ExitCode(_) => "exit_code",
        _ => "invalid_result",
    }
}

#[cfg(test)]
mod tests;
