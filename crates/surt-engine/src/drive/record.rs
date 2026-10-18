use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::driver::Driver;
use super::frozen::gate_artifacts;
use super::guard::refused;
use super::load::{Loaded, label};
use super::outcome::Progress;
use crate::artifacts::{AcceptedDelivery, ValidatedOutput};
use crate::project::{Error, Refusal};
use crate::runtime::AgentRuntime;
use crate::store::{Acceptance, ExecutionEnd, PublishedOutput, StartedExecution, StepRow};
use crate::workflow::Definition;
use crate::working_copy::RecordedEdit;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn publish(
        &self,
        loaded: &Loaded,
        definition: &Definition,
        row: &StepRow,
        started: &StartedExecution,
        delivery: AcceptedDelivery,
    ) -> Result<Option<Refusal>, Error> {
        let in_root = row.run == loaded.rows.id;
        let outputs = published_outputs(definition, in_root, delivery.outputs);
        let saved = match refused(self.save_edits(&outputs).await)? {
            Ok(saved) => saved,
            Err(refusal) => {
                self.end(row, started, ExecutionEnd::Interrupted).await?;
                return Ok(Some(refusal));
            }
        };
        let acceptance = Acceptance {
            execution: &started.id,
            outcome: &delivery.outcome,
            outputs: &outputs,
        };
        let published = self.driving.store.accept_execution(acceptance).await?;
        let working_copy = self.driving.layout.working_copy(&loaded.rows.subject);
        for (output, (name, revision)) in outputs.iter().zip(published) {
            if output.working_copy {
                self.materialize(&working_copy, &row.run, output, &revision.revision_id)
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
            step: label(row),
            outcome: delivery.outcome,
        });
        Ok(None)
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
    definition: &Definition,
    in_root: bool,
    outputs: BTreeMap<String, ValidatedOutput>,
) -> Vec<PublishedOutput> {
    let gated = gate_artifacts(definition);
    outputs
        .into_iter()
        .map(|(name, output)| PublishedOutput {
            working_copy: in_root && gated.contains(name.as_str()),
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
