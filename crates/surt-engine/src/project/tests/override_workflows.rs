use super::fixtures::{ProjectDir, agent, default_config, new_run, override_refusal, writer};
use crate::project::{
    Catalog, DefinitionFile, DeliveryCheck, Error, InvalidDefinition, InvalidOverride, Problem,
    Refusal, WorkflowStep,
};

const UNTIL_DONE: &str = r#"
schema = 1
name = "drafting"
description = "Write until the writer is done."

[steps.write]
kind = "agent"
agent = "writer"

[steps.cycle]
kind = "loop"
body = ["write"]
until = { step = "write", outcome = "done" }
"#;

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

const WRITER_FILE: &str = ".surt/agents/writer/agent.toml";

fn drafting() -> Catalog {
    Catalog::new()
        .workflow("drafting", UNTIL_DONE)
        .agent("writer", writer())
}

async fn refused(dir: &ProjectDir, catalog: Catalog) -> InvalidOverride {
    override_refusal(dir, catalog, "drafting").await
}

fn step(name: &str) -> Option<WorkflowStep> {
    Some(WorkflowStep {
        workflow: "drafting".to_owned(),
        step: name.to_owned(),
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_removing_an_outcome_a_workflow_waits_for_refuses_naming_the_step() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", "outcomes = [\"finished\"]\n");

    let invalid = refused(&dir, drafting()).await;

    assert_eq!(
        invalid,
        InvalidOverride {
            file: WRITER_FILE.to_owned(),
            field: None,
            needed_by: step("cycle"),
            problem: Problem::UnknownOutcome {
                step: "write".to_owned(),
                outcome: "done".to_owned(),
                declared: vec!["finished".to_owned()],
            },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_override_requiring_an_input_the_workflow_never_supplies_refuses_naming_the_step() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[inputs.style]\nformat = \"text\"\n",
    );

    let invalid = refused(&dir, drafting()).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.needed_by, invalid.problem),
        (
            WRITER_FILE,
            step("write"),
            Problem::MissingInput {
                input: "style".to_owned()
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
        "outcomes = [\"done\", \"blocked\"]
",
    );
    let catalog = Catalog::new()
        .workflow("drafting", UNTIL_DONE)
        .agent("writer", agent(APPROVING_WRITER))
        .delivery_check(
            "writer",
            "notes",
            DeliveryCheck::NoBlockingFinding {
                outcome: "approved",
            },
        );

    let invalid = refused(&dir, catalog).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.needed_by, invalid.problem),
        (
            WRITER_FILE,
            step("write"),
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
