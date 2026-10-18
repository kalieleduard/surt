use std::fs::{self, OpenOptions};
use std::path::Path;
use std::sync::atomic::Ordering;

use tokio::sync::mpsc;
use uuid::Uuid;

use super::driver::{Driver, known};
use super::frozen::{CorruptSnapshot, FrozenAgent};
use super::graph::{StepRunId, StepRunState};
use super::inputs::scoped;
use super::load::{Loaded, label};
use super::outcome::Progress;
use super::record::WorkingCopy;
use crate::artifacts::{AcceptedDelivery, DeliveryContract};
use crate::context::{Assembly, ContextInput, PreviousIteration};
use crate::project::Error;
use crate::runtime::{
    AgentRuntime, Rejection, RuntimeRequest, StepAttempt, StopLevel, Verdict, accept,
};
use crate::store::{ExecutionEnd, ExecutionStart, StartedExecution, StepRow};
use crate::workflow::StepSpec;

const DELIVERY_DIR: &str = "delivery";
const LAUNCH_DIR: &str = "launch";
const LOG_FILES: [&str; 2] = ["stream.jsonl", "stderr.log"];
const SKIPPED: &str = "SKIPPED";
const FIRST_ITERATION: u32 = 1;

pub(super) struct Launch {
    pub(super) slot: usize,
    pub(super) row: StepRow,
    pub(super) started: StartedExecution,
    request: RuntimeRequest,
    contract: DeliveryContract,
    pub(super) working_copy: Option<WorkingCopy>,
}

enum Ending {
    Accepted(AcceptedDelivery),
    Ended(ExecutionEnd),
    Cancelled,
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
            agent_snapshot: &agent.snapshot,
            slot,
            inputs: self.input_requests(loaded, row, spec, agent)?,
        };
        let started = self.driving.store.start_execution(start).await?;
        let request = self.prepare(loaded, row, agent, &started).await?;
        self.report(Progress::StepStarted {
            step: label(row),
            attempt: started.attempt,
        });
        Ok(Launch {
            slot,
            row: row.clone(),
            started,
            request,
            contract,
            working_copy: self.working_copy(loaded, row, definition),
        })
    }

    pub(super) async fn carry(&self, launch: Launch) -> Result<(), Error> {
        match self.execute(&launch.request, &launch.contract).await {
            Ending::Accepted(delivery) => self.publish(&launch, delivery).await,
            Ending::Ended(end) => self.end(&launch.row, &launch.started, end).await,
            Ending::Cancelled => Ok(()),
        }
    }

    async fn prepare(
        &self,
        loaded: &Loaded,
        row: &StepRow,
        agent: &FrozenAgent,
        started: &StartedExecution,
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
            launch: agent.launch(layout.root().to_owned(), execution_dir.join(LAUNCH_DIR))?,
        })
    }

    async fn execute(&self, request: &RuntimeRequest, contract: &DeliveryContract) -> Ending {
        let (events, _received) = mpsc::unbounded_channel();
        let result = self
            .runtime
            .execute(request.clone(), events, self.agents.clone())
            .await;
        if self.cancelling.load(Ordering::SeqCst) {
            return Ending::Cancelled;
        }
        if self.agents.level() != StopLevel::None {
            return Ending::Ended(ExecutionEnd::Interrupted);
        }
        let completion = match result {
            Ok(completion) => completion,
            Err(error) => {
                return Ending::Ended(ExecutionEnd::Failed {
                    class: error.class,
                    code: error.code,
                    message: error.message,
                    resets_at: error.resets_at,
                });
            }
        };
        match accept(request, &completion, contract) {
            Verdict::Accepted(delivery) => Ending::Accepted(delivery),
            Verdict::Interrupted => Ending::Ended(ExecutionEnd::Interrupted),
            Verdict::Rejected(rejection) => Ending::Ended(ExecutionEnd::Failed {
                class: rejection.class(),
                code: rejection_code(&rejection).to_owned(),
                message: format!("{rejection:?}"),
                resets_at: None,
            }),
        }
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
