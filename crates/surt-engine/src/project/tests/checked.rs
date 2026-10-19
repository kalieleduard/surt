use super::fixtures::{Catalog, ProjectDir, draft_catalog, writer};
use crate::project::{DefinitionFile, Error, InvalidDefinition, Problem, Refusal, check_workflow};

const INITIAL_ARTIFACTS: [&str; 2] = ["request", "base"];

fn checked(catalog: &Catalog, initial_artifacts: &[&str]) -> Result<(), Refusal> {
    let dir = ProjectDir::new();
    catalog.write(&dir.path().join(".surt"));
    check_workflow(&dir.path(), "draft", initial_artifacts).map_err(|error| match error {
        Error::Refused(refusal) => refusal,
        Error::Internal(internal) => panic!("expected a refusal, got {internal}"),
    })
}

#[test]
fn definitions_on_disk_whose_workflow_and_agents_are_valid_pass_the_static_check() {
    assert_eq!(checked(&draft_catalog(), &INITIAL_ARTIFACTS), Ok(()));
}

#[test]
fn the_static_check_needs_the_initial_artifacts_the_creating_command_supplies() {
    assert_eq!(
        checked(&draft_catalog(), &["base"]),
        Err(Refusal::InvalidDefinition(Box::new(InvalidDefinition {
            file: DefinitionFile::Workflow("draft".to_owned()),
            field: None,
            step: Some("write".to_owned()),
            problem: Problem::MissingInput {
                input: "request".to_owned(),
            },
        })))
    );
}

#[test]
fn an_unknown_workflow_fails_the_static_check() {
    let catalog = Catalog::new().agent("writer", writer());

    assert_eq!(
        checked(&catalog, &INITIAL_ARTIFACTS),
        Err(Refusal::UnknownWorkflow {
            name: "draft".to_owned(),
            known: Vec::new(),
        })
    );
}
