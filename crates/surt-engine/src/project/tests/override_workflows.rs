use super::fixtures::{ProjectDir, override_refusal, writer};
use crate::project::{Catalog, InvalidOverride, Problem, WorkflowStep};

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
