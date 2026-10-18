use std::path::Path;
use std::time::Duration;

use super::driver::{Driver, known};
use super::failure::OperationError;
use super::frozen::CorruptSnapshot;
use super::graph::{OperationId, StepRunId};
use super::launch::Launch;
use super::load::{Loaded, label};
use super::outcome::Progress;
use crate::operations::Operation;
use crate::platform::{self, ExclusiveLock};
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::{
    Acceptance, ExecutionKind, ExecutionStart, Integrated, PublishedOutput, StartedExecution,
    StepRow,
};

const GIT_LOCK_POLL: Duration = Duration::from_millis(10);

pub(super) enum Flight {
    Agent(Box<Launch>),
    Operation(Box<OperationLaunch>),
}

pub(super) struct OperationLaunch {
    pub(super) row: StepRow,
    pub(super) started: StartedExecution,
    pub(super) operation: Operation,
    pub(super) subject: String,
    pub(super) ordinal: u32,
    pub(super) root: String,
}

pub(super) struct Finished {
    pub(super) outcome: &'static str,
    pub(super) outputs: Vec<PublishedOutput>,
    pub(super) merged: Option<String>,
}

impl From<&'static str> for Finished {
    fn from(outcome: &'static str) -> Finished {
        Finished {
            outcome,
            outputs: Vec::new(),
            merged: None,
        }
    }
}

impl Flight {
    pub(super) fn slot(&self) -> Option<usize> {
        match self {
            Flight::Agent(launch) => Some(launch.slot),
            Flight::Operation(_) => None,
        }
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn fly(&self, flight: Flight) -> Result<(), Error> {
        match flight {
            Flight::Agent(launch) => self.carry(*launch).await,
            Flight::Operation(launch) => self.operate(*launch).await,
        }
    }

    pub(super) async fn start_operation(
        &self,
        loaded: &Loaded,
        step: &StepRunId,
        op: &OperationId,
    ) -> Result<OperationLaunch, Error> {
        let row = known(loaded, step)?;
        let operation = Operation::find(&op.0)
            .ok_or_else(|| CorruptSnapshot::new(&row.key, "names an unknown operation"))?;
        let start = ExecutionStart {
            step: &row.id,
            kind: ExecutionKind::Operation { id: operation.id() },
            inputs: self.input_requests(
                loaded,
                row,
                operation.inputs().iter().map(|input| (*input, *input)),
            )?,
        };
        let started = self.driving.store.start_execution(start).await?;
        self.report(Progress::StepStarted {
            step: label(row),
            attempt: started.attempt,
        });
        Ok(OperationLaunch {
            row: row.clone(),
            started,
            operation,
            subject: loaded.rows.subject.clone(),
            ordinal: loaded.rows.ordinal,
            root: loaded.rows.id.clone(),
        })
    }

    async fn operate(&self, launch: OperationLaunch) -> Result<(), Error> {
        let _serialized = match launch.operation {
            Operation::Integrate => Some(self.branches.lock().await),
            _ => None,
        };
        let result = match launch.operation {
            Operation::CreateRunBranch => self.create_run_branch(&launch).await.map(Finished::from),
            Operation::CreateChildWorktree => self
                .create_child_worktree(&launch)
                .await
                .map(Finished::from),
            Operation::RunChecks => self.run_checks(&launch).await,
            Operation::Integrate => self.integrate(&launch).await,
            Operation::SnapshotRunBranch => self.snapshot_run_branch(&launch).await,
        };
        match result {
            Ok(finished) => self.finish_operation(&launch, finished).await,
            Err(OperationError::Failed(failure)) => {
                self.end(&launch.row, &launch.started, failure.into_end())
                    .await
            }
            Err(OperationError::Fatal(error)) => Err(error),
        }
    }

    async fn finish_operation(
        &self,
        launch: &OperationLaunch,
        finished: Finished,
    ) -> Result<(), Error> {
        let acceptance = Acceptance {
            execution: &launch.started.id,
            outcome: finished.outcome,
            outputs: &finished.outputs,
            tip: None,
            integrated: finished.merged.as_deref().map(|merge| Integrated {
                into: &launch.root,
                merge,
            }),
        };
        let published = self.driving.store.accept_execution(acceptance).await?;
        for (artifact, revision) in published {
            self.report(Progress::RevisionPublished {
                artifact,
                revision: revision.number,
            });
        }
        self.report(Progress::StepFinished {
            step: label(&launch.row),
            outcome: finished.outcome.to_owned(),
        });
        Ok(())
    }

    pub(super) async fn git_lock(&self) -> Result<ExclusiveLock, Error> {
        let path = self.driving.layout.git_lock();
        loop {
            if let Some(lock) = try_lock(&path)? {
                return Ok(lock);
            }
            tokio::time::sleep(GIT_LOCK_POLL).await;
        }
    }
}

fn try_lock(path: &Path) -> Result<Option<ExclusiveLock>, Error> {
    platform::try_lock_exclusive(path).map_err(|error| Error::io("lock", path, error))
}
