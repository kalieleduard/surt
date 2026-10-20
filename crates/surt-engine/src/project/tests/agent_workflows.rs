use super::fixtures::{Catalog, ProjectDir, WRITER_AGENT, default_config, invalid_file, writer};
use crate::project::{DefinitionFile, InvalidDefinition, Problem, StepField};

const UNTIL_DONE: &str = r#"
schema = 1
name = "drafting"
description = "Write until the writer is done."

[inputs.request]
format = "text"

[steps.write]
kind = "agent"
agent = "writer"

[steps.cycle]
kind = "loop"
body = ["write"]
until = { step = "write", outcome = "done" }
"#;

const ONCE: &str = r#"
schema = 1
name = "drafting"

[inputs.request]
format = "text"

[steps.write]
kind = "agent"
agent = "writer"
"#;

async fn refused(agent_toml: &str) -> InvalidDefinition {
    refused_in(UNTIL_DONE, agent_toml).await
}

async fn refused_in(workflow: &'static str, agent_toml: &str) -> InvalidDefinition {
    let dir = ProjectDir::new();
    let catalog = Catalog::new()
        .workflow("drafting", workflow)
        .agent("writer", writer());
    dir.project(catalog, default_config()).await;
    dir.write_agent_file("writer", "agent.toml", agent_toml);
    invalid_file(&dir, "drafting").await
}

fn at_step(step: &str, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Workflow("drafting".to_owned()),
        field: None,
        step: Some(step.to_owned()),
        problem,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_without_the_outcome_a_workflow_waits_for_refuses_at_the_workflow_step() {
    let finished = WRITER_AGENT.replace(
        "id = \"writer\"",
        "id = \"writer\"\noutcomes = [\"finished\"]",
    );

    let invalid = refused(&finished).await;

    assert_eq!(
        invalid,
        at_step(
            "cycle",
            Problem::UnknownOutcome {
                field: StepField::Until,
                step: "write".to_owned(),
                outcome: "done".to_owned(),
                declared: vec!["finished".to_owned()],
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_requiring_an_input_the_workflow_never_supplies_refuses_at_the_step() {
    let styled = WRITER_AGENT.replace(
        "[inputs.request]",
        "[inputs.style]\nformat = \"text\"\n\n[inputs.request]",
    );

    let invalid = refused(&styled).await;

    assert_eq!(
        invalid,
        at_step(
            "write",
            Problem::MissingInput {
                input: "style".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_continuing_its_conversation_refuses_at_a_step_outside_every_loop() {
    let continuing = WRITER_AGENT.replace("[profile]", "[profile]\nconversation = \"loop\"");

    let invalid = refused_in(ONCE, &continuing).await;

    assert_eq!(
        invalid,
        at_step(
            "write",
            Problem::ConversationOutsideLoop {
                agent: "writer".to_owned()
            }
        )
    );
}
