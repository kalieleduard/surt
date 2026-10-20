use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use uuid::Uuid;

use super::driver::Driver;
use super::frozen::FrozenAgent;
use super::graph::StepRunState;
use super::inputs::scoped;
use super::load::Loaded;
use crate::context::{Assembly, ContextInput, PreviousIteration};
use crate::project::{Error, Layout};
use crate::runtime::{AgentRuntime, RuntimeRequest, STDERR_LOG, STREAM_LOG, StepAttempt};
use crate::store::{RunBranch, StartedExecution, StepRow, StoredRevision};

const DELIVERY_DIR: &str = "delivery";
const LAUNCH_DIR: &str = "launch";
const LOG_FILES: [&str; 2] = [STREAM_LOG, STDERR_LOG];
const SKIPPED: &str = "SKIPPED";
const FIRST_ITERATION: u32 = 1;

struct Directories {
    execution: PathBuf,
    delivery: PathBuf,
    logs: PathBuf,
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn prepare(
        &self,
        loaded: &Loaded,
        row: &StepRow,
        agent: &FrozenAgent,
        started: &StartedExecution,
        worktree: Option<&RunBranch>,
    ) -> Result<RuntimeRequest, Error> {
        let layout = self.driving.layout;
        let directories = Directories::create(layout, &loaded.rows.subject, &started.id)?;
        let revisions = self.revisions(row, started).await?;
        let assembly = Assembly {
            execution_dir: &directories.execution,
            delivery_dir: &directories.delivery,
            previous: previous_iteration(loaded, row),
            inputs: revisions
                .iter()
                .map(|(name, revision)| ContextInput { name, revision })
                .collect(),
        };
        let context = assembly
            .write()
            .map_err(|error| Error::io("write the context in", &directories.execution, error))?;
        let continuation = self.continuation(row, agent, started, &assembly).await?;
        let working_dir = worktree
            .and_then(|branch| branch.worktree_path.clone())
            .unwrap_or_else(|| layout.root().to_owned());
        Ok(RuntimeRequest {
            step: step_attempt(row, started),
            session_id: Uuid::now_v7(),
            model: agent.contract.model().to_owned(),
            launch: agent.launch(working_dir, directories.execution.join(LAUNCH_DIR))?,
            delivery_dir: directories.delivery,
            log_dir: directories.logs,
            context,
            continuation,
        })
    }

    async fn revisions<'s>(
        &self,
        row: &StepRow,
        started: &'s StartedExecution,
    ) -> Result<Vec<(&'s str, StoredRevision)>, Error> {
        let mut revisions = Vec::new();
        for (name, revision) in &started.inputs {
            let revision = self.driving.store.revision(revision).await?;
            revisions.push((name.as_str(), scoped(name, row, revision)?));
        }
        Ok(revisions)
    }
}

impl Directories {
    fn create(layout: &Layout, subject: &str, execution: &str) -> Result<Directories, Error> {
        let execution_dir = layout.execution(execution);
        let delivery = execution_dir.join(DELIVERY_DIR);
        let logs = layout.execution_logs(subject, execution);
        fs::create_dir_all(&delivery).map_err(|error| Error::io("create", &delivery, error))?;
        create_logs(&logs).map_err(|error| Error::io("create", &logs, error))?;
        Ok(Directories {
            execution: execution_dir,
            delivery,
            logs,
        })
    }
}

fn step_attempt(row: &StepRow, started: &StartedExecution) -> StepAttempt {
    let key = match &row.item {
        Some(item) => format!("{item}/{}", row.key),
        None => row.key.clone(),
    };
    StepAttempt::new(
        &key,
        row.iteration.unwrap_or(FIRST_ITERATION),
        started.attempt,
    )
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
