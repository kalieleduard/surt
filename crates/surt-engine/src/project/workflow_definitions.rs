use super::create_run::declared_inputs;
use super::definitions::ProjectDefinitions;
use super::{DeclaredInput, Error, Project};
use crate::workflow::validate;

#[derive(Debug)]
pub struct WorkflowDefinitionSummary {
    pub name: String,
    pub run_inputs: Result<Vec<DeclaredInput>, Error>,
}

impl Project {
    pub fn workflow_definitions(&self) -> Result<Vec<WorkflowDefinitionSummary>, Error> {
        let definitions = ProjectDefinitions::new(&self.layout);
        let names = definitions.workflow_names()?;
        Ok(names
            .into_iter()
            .map(|name| WorkflowDefinitionSummary {
                run_inputs: validate(&definitions, &name)
                    .map(|inputs| declared_inputs(&inputs))
                    .map_err(Error::from),
                name,
            })
            .collect())
    }
}
