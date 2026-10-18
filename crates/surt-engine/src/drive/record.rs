use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::driver::Driver;
use super::frozen::gate_artifacts;
use super::launch::Launch;
use super::load::{Loaded, label};
use super::outcome::Progress;
use crate::artifacts::AcceptedDelivery;
use crate::git;
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::{Acceptance, ExecutionEnd, PublishedOutput, StartedExecution, StepRow};
use crate::workflow::Definition;

pub(super) struct WorkingCopy {
    dir: PathBuf,
    artifacts: BTreeSet<String>,
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn working_copy(
        &self,
        loaded: &Loaded,
        row: &StepRow,
        definition: &Definition,
    ) -> Option<WorkingCopy> {
        (row.run == loaded.rows.id).then(|| WorkingCopy {
            dir: self.driving.layout.working_copy(&loaded.rows.subject),
            artifacts: gate_artifacts(definition)
                .into_iter()
                .map(str::to_owned)
                .collect(),
        })
    }

    pub(super) async fn publish(
        &self,
        launch: &Launch,
        delivery: AcceptedDelivery,
    ) -> Result<(), Error> {
        let gated = |name: &str| {
            launch
                .working_copy
                .as_ref()
                .is_some_and(|copy| copy.artifacts.contains(name))
        };
        let outputs: Vec<PublishedOutput> = delivery
            .outputs
            .into_iter()
            .map(|(name, output)| PublishedOutput {
                working_copy: gated(&name),
                name,
                files: output
                    .files
                    .into_iter()
                    .map(|file| (file.path, file.bytes))
                    .collect(),
            })
            .collect();
        let tip = match &launch.worktree {
            Some(branch) => git::commit_sha(self.driving.layout.root(), &branch.ref_name).await?,
            None => None,
        };
        let acceptance = Acceptance {
            execution: &launch.started.id,
            outcome: &delivery.outcome,
            outputs: &outputs,
            tip: tip.as_deref(),
        };
        let published = self.driving.store.accept_execution(acceptance).await?;
        for (output, (name, revision)) in outputs.iter().zip(published) {
            if let Some(copy) = launch.working_copy.as_ref().filter(|_| output.working_copy) {
                write_files(&copy.dir, &output.files)
                    .map_err(|error| Error::io("write", &copy.dir, error))?;
                self.driving
                    .store
                    .promote_working_copy(&launch.row.run, &revision.revision_id)
                    .await?;
            }
            self.report(Progress::RevisionPublished {
                artifact: name,
                revision: revision.number,
            });
        }
        self.report(Progress::StepFinished {
            step: label(&launch.row),
            outcome: delivery.outcome,
        });
        Ok(())
    }

    pub(super) async fn end(
        &self,
        row: &StepRow,
        started: &StartedExecution,
        end: ExecutionEnd,
    ) -> Result<(), Error> {
        self.driving.store.end_execution(&started.id, &end).await?;
        let step = label(row);
        let attempt = started.attempt;
        self.report(match end {
            ExecutionEnd::Failed { code, message, .. } => Progress::AttemptFailed {
                step,
                attempt,
                code,
                message,
            },
            ExecutionEnd::Interrupted => Progress::AttemptInterrupted { step, attempt },
        });
        Ok(())
    }
}

fn write_files(root: &Path, files: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    for (rel_path, bytes) in files {
        let target = root.join(rel_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, bytes)?;
    }
    Ok(())
}
