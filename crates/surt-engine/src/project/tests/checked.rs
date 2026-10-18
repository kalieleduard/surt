use super::fixtures::{draft_catalog, writer};
use crate::project::{Catalog, DefinitionFile, InvalidDefinition, Problem};

const INITIAL_ARTIFACTS: [&str; 2] = ["request", "base"];

#[test]
fn a_catalog_whose_workflow_and_agents_are_valid_passes_the_static_check() {
    assert_eq!(draft_catalog().check("draft", &INITIAL_ARTIFACTS), Ok(()));
}

#[test]
fn the_static_check_needs_the_initial_artifacts_the_creating_command_supplies() {
    let checked = draft_catalog().check("draft", &["base"]);

    assert_eq!(
        checked,
        Err(Box::new(InvalidDefinition {
            file: DefinitionFile::Workflow("draft".to_owned()),
            step: Some("write".to_owned()),
            problem: Problem::MissingInput {
                input: "request".to_owned(),
            },
        }))
    );
}

#[test]
fn an_unknown_workflow_fails_the_static_check() {
    let catalog = Catalog::new().agent("writer", writer());

    let checked = catalog.check("draft", &INITIAL_ARTIFACTS);

    assert!(
        matches!(&checked, Err(invalid) if invalid.problem == Problem::UnknownWorkflow { name: "draft".to_owned() }),
        "{checked:?}"
    );
}
