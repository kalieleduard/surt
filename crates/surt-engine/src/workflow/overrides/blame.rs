use super::{InvalidOverride, LocalAgent, LocalFile, StepUse, WorkflowStep};
use crate::workflow::catalog::Catalog;
use crate::workflow::error::{DefinitionFile, InvalidDefinition, Problem, StepField};

impl Catalog {
    pub(crate) fn blame_overrides(
        &self,
        effective: &Catalog,
        locals: &[LocalAgent],
        workflow: &str,
        initial_artifacts: &[&str],
    ) -> Result<(), Box<InvalidOverride>> {
        let check = |catalog: &Catalog| catalog.check(workflow, initial_artifacts);
        if check(effective).is_ok() || check(self).is_err() {
            return Ok(());
        }
        let mut applied = self.clone();
        for local in locals {
            let Some(file) = override_file(local) else {
                continue;
            };
            applied = applied.with_override_from(effective, &local.id);
            if let Err(invalid) = check(&applied) {
                return Err(blamed(file, *invalid));
            }
        }
        Ok(())
    }
}

fn override_file(local: &LocalAgent) -> Option<&LocalFile> {
    local
        .toml
        .as_ref()
        .or(local.instructions.as_ref())
        .or_else(|| local.schemas.values().next())
}

fn blamed(file: &LocalFile, invalid: InvalidDefinition) -> Box<InvalidOverride> {
    Box::new(InvalidOverride {
        file: file.file.clone(),
        field: None,
        needed_by: needed_by(&invalid),
        problem: invalid.problem,
    })
}

fn needed_by(invalid: &InvalidDefinition) -> Option<WorkflowStep> {
    match (&invalid.file, &invalid.step) {
        (DefinitionFile::Workflow(workflow), Some(step)) => Some(WorkflowStep {
            workflow: workflow.clone(),
            step: step.clone(),
            field: step_use(&invalid.problem),
        }),
        _ => None,
    }
}

fn step_use(problem: &Problem) -> Option<StepUse> {
    match problem {
        Problem::UnknownOutcome {
            field: StepField::Until,
            ..
        } => Some(StepUse::Until),
        Problem::UnknownOutcome { .. } => Some(StepUse::When),
        Problem::UnknownAgentInput { .. } | Problem::MissingInput { .. } => Some(StepUse::Input),
        Problem::MissingCheckedOutput { .. } | Problem::MissingCheckedOutcome { .. } => {
            Some(StepUse::DeliveryCheck)
        }
        _ => None,
    }
}
