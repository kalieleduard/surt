use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::driver::Driver;
use super::frozen::gate_artifacts;
use super::guard::refused;
use super::launch::Launch;
use super::load::{Loaded, label};
use super::outcome::Progress;
use crate::artifacts::working_copy::RecordedEdit;
use crate::artifacts::{AcceptedDelivery, ValidatedOutput};
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
        let outputs = published_outputs(launch, delivery.outputs);
        let Ok(saved) = refused(self.save_edits(&outputs).await)? else {
            return self
                .end(&launch.row, &launch.started, ExecutionEnd::Interrupted)
                .await;
        };
        let tip = match &launch.worktree {
            Some(branch) => git::commit_sha(self.driving.layout.root(), &branch.ref_name).await?,
            None => None,
        };
        let acceptance = Acceptance {
            execution: &launch.started.id,
            outcome: &delivery.outcome,
            outputs: &outputs,
            tip: tip.as_deref(),
            integrated: None,
        };
        let published = self.driving.store.accept_execution(acceptance).await?;
        for (output, (name, revision)) in outputs.iter().zip(published) {
            if let Some(copy) = launch.working_copy.as_ref().filter(|_| output.working_copy) {
                self.materialize(&copy.dir, &launch.row.run, output, &revision.revision_id)
                    .await?;
            }
            if let Some(edit) = saved.iter().find(|edit| edit.artifact == name) {
                self.report(Progress::EditSuperseded {
                    artifact: name.clone(),
                    edit: edit.revision,
                    by: revision.number,
                });
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

    async fn materialize(
        &self,
        working_copy: &Path,
        run: &str,
        output: &PublishedOutput,
        revision: &str,
    ) -> Result<(), Error> {
        let store = self.driving.store;
        let superseded = store.superseded_working_copy_files(revision).await?;
        remove_files(working_copy, &superseded)
            .and_then(|()| write_files(working_copy, &output.files))
            .map_err(|error| Error::io("write", working_copy, error))?;
        Ok(store.promote_working_copy(run, revision).await?)
    }

    async fn save_edits(&self, outputs: &[PublishedOutput]) -> Result<Vec<RecordedEdit>, Error> {
        let artifacts: Vec<&str> = outputs
            .iter()
            .filter(|output| output.working_copy)
            .map(|output| output.name.as_str())
            .collect();
        let scanned = self
            .working_copy
            .record_artifacts(self.driving.store, self.driving.actor, &artifacts)
            .await?;
        Ok(scanned.recorded)
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

fn published_outputs(
    launch: &Launch,
    outputs: BTreeMap<String, ValidatedOutput>,
) -> Vec<PublishedOutput> {
    let gated = |name: &str| {
        launch
            .working_copy
            .as_ref()
            .is_some_and(|copy| copy.artifacts.contains(name))
    };
    outputs
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
        .collect()
}

fn remove_files(root: &Path, rel_paths: &[String]) -> std::io::Result<()> {
    for rel_path in rel_paths {
        match fs::remove_file(root.join(rel_path)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
    }
    Ok(())
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
