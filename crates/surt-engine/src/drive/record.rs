use std::fs;
use std::path::Path;

use super::driver::Driver;
use super::frozen::gate_artifacts;
use super::load::{Loaded, label};
use super::outcome::Progress;
use crate::artifacts::AcceptedDelivery;
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::{Acceptance, ExecutionEnd, PublishedOutput, StartedExecution, StepRow};
use crate::workflow::Definition;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn publish(
        &self,
        loaded: &Loaded,
        definition: &Definition,
        row: &StepRow,
        started: &StartedExecution,
        delivery: AcceptedDelivery,
    ) -> Result<(), Error> {
        let gated = gate_artifacts(definition);
        let in_root = row.run == loaded.rows.id;
        let outputs: Vec<PublishedOutput> = delivery
            .outputs
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
            .collect();
        let acceptance = Acceptance {
            execution: &started.id,
            outcome: &delivery.outcome,
            outputs: &outputs,
        };
        let published = self.driving.store.accept_execution(acceptance).await?;
        let working_copy = self.driving.layout.working_copy(&loaded.rows.subject);
        for (output, (name, revision)) in outputs.iter().zip(published) {
            if output.working_copy {
                write_files(&working_copy, &output.files)
                    .map_err(|error| Error::io("write", &working_copy, error))?;
                self.driving
                    .store
                    .promote_working_copy(&row.run, &revision.revision_id)
                    .await?;
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
