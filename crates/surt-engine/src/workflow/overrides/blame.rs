use super::{InvalidOverride, LocalAgent, LocalFile, WorkflowStep};
use crate::workflow::catalog::Catalog;
use crate::workflow::error::{DefinitionFile, InvalidDefinition};

impl Catalog {
    pub(crate) fn blame_overrides(
        &self,
        effective: &Catalog,
        locals: &[LocalAgent],
        workflow: &str,
        initial_artifacts: &[&str],
    ) -> Result<(), Box<InvalidOverride>> {
        let Err(invalid) = effective.check(workflow, initial_artifacts) else {
            return Ok(());
        };
        let built_in_valid = self.check(workflow, initial_artifacts).is_ok();
        let overridden: Vec<(&str, &LocalFile)> = locals
            .iter()
            .filter_map(|local| local.toml.as_ref().map(|file| (local.id.as_str(), file)))
            .collect();
        let culprit = overridden
            .iter()
            .find(|(id, _)| {
                self.with_one_override(effective, id)
                    .check(workflow, initial_artifacts)
                    .is_err()
            })
            .or(overridden.first());
        match culprit {
            Some((_, file)) if built_in_valid => Err(Box::new(InvalidOverride {
                file: file.file.clone(),
                field: None,
                needed_by: needed_by(&invalid),
                problem: invalid.problem,
            })),
            _ => Ok(()),
        }
    }

    fn with_one_override(&self, effective: &Catalog, id: &str) -> Catalog {
        match effective.agent_toml(id) {
            Some(toml) => self.clone().with_effective_toml(id, toml.to_owned()),
            None => self.clone(),
        }
    }
}

fn needed_by(invalid: &InvalidDefinition) -> Option<WorkflowStep> {
    match (&invalid.file, &invalid.step) {
        (DefinitionFile::Workflow(workflow), Some(step)) => Some(WorkflowStep {
            workflow: workflow.clone(),
            step: step.clone(),
        }),
        _ => None,
    }
}
