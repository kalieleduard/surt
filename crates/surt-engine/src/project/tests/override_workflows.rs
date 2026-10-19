use super::fixtures::{ProjectDir, agent, override_refusal, writer};
use crate::project::{Catalog, InvalidOverride, Problem, StepField, StepUse, WorkflowStep};

pub(super) const UNTIL_DONE: &str = r#"
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

pub(super) const WRITER_FILE: &str = ".surt/agents/writer/agent.toml";

pub(super) fn drafting() -> Catalog {
    Catalog::new()
        .workflow("drafting", UNTIL_DONE)
        .agent("writer", writer())
}

pub(super) async fn refused(dir: &ProjectDir, catalog: Catalog) -> InvalidOverride {
    override_refusal(dir, catalog, "drafting").await
}

pub(super) fn step(name: &str, field: StepUse) -> Option<WorkflowStep> {
    Some(WorkflowStep {
        workflow: "drafting".to_owned(),
        step: name.to_owned(),
        field: Some(field),
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
            needed_by: step("cycle", StepUse::Until),
            problem: Problem::UnknownOutcome {
                field: StepField::Until,
                step: "write".to_owned(),
                outcome: "done".to_owned(),
                declared: vec!["finished".to_owned()],
            },
        }
    );
}

const EDITOR: &str = r#"
schema = 1
id = "editor"
outcomes = ["done"]

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

#[tokio::test(flavor = "multi_thread")]
async fn with_several_overrides_the_refusal_names_the_one_that_breaks_the_workflow() {
    let dir = ProjectDir::new();
    dir.write_local("editor", "agent.toml", "[profile]\neffort = \"low\"\n");
    dir.write_local("editor", "instructions.md", "Edit.\n");
    dir.write_local("writer", "agent.toml", "outcomes = [\"finished\"]\n");

    let invalid = refused(&dir, drafting().agent("editor", agent(EDITOR))).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.needed_by),
        (WRITER_FILE, step("cycle", StepUse::Until))
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
            step("write", StepUse::Input),
            Problem::MissingInput {
                input: "style".to_owned()
            }
        )
    );
}
