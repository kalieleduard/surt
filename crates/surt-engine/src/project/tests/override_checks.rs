use super::fixtures::{ProjectDir, agent, default_config, new_run};
use super::override_workflows::{UNTIL_DONE, WRITER_FILE, drafting, refused, step};
use crate::project::{
    Catalog, DefinitionFile, DeliveryCheck, Error, InvalidDefinition, Problem, Refusal, StepUse,
};

const APPROVING_WRITER: &str = r#"
schema = 1
id = "writer"
outcomes = ["done", "approved"]

[inputs.request]
format = "text"

[outputs.notes]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

fn checked_writer(check: DeliveryCheck) -> Catalog {
    Catalog::new()
        .workflow("drafting", UNTIL_DONE)
        .agent("writer", agent(APPROVING_WRITER))
        .delivery_check("writer", "notes", check)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_that_stops_delivering_a_checked_output_as_json_refuses_naming_the_step() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[outputs.notes]\nformat = \"text\"\n",
    );

    let invalid = refused(&dir, checked_writer(DeliveryCheck::CommitEvidence)).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.needed_by, invalid.problem),
        (
            WRITER_FILE,
            step("write", StepUse::DeliveryCheck),
            Problem::MissingCheckedOutput {
                output: "notes".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_removing_the_outcome_a_delivery_check_names_refuses_naming_the_step() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "outcomes = [\"done\", \"blocked\"]\n",
    );
    let catalog = checked_writer(DeliveryCheck::NoBlockingFinding {
        outcome: "approved",
    });

    let invalid = refused(&dir, catalog).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.needed_by, invalid.problem),
        (
            WRITER_FILE,
            step("write", StepUse::DeliveryCheck),
            Problem::MissingCheckedOutcome {
                outcome: "approved".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delivery_check_on_an_output_the_agent_does_not_deliver_as_json_is_an_invalid_definition()
{
    let dir = ProjectDir::new();
    let catalog = drafting().delivery_check("writer", "draft", DeliveryCheck::CommitEvidence);

    let result = dir
        .project(catalog, default_config())
        .await
        .create_run(new_run("drafting"))
        .await;

    match result {
        Err(Error::Refused(Refusal::InvalidDefinition(invalid))) => assert_eq!(
            *invalid,
            InvalidDefinition {
                file: DefinitionFile::Workflow("drafting".to_owned()),
                step: Some("write".to_owned()),
                problem: Problem::MissingCheckedOutput {
                    output: "draft".to_owned()
                },
            }
        ),
        other => panic!("expected an invalid definition, got {other:?}"),
    }
}
