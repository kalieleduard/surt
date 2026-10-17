use super::fixtures::{ProjectDir, default_config, in_step, new_run, workflow_refusal, writer};
use crate::project::{Catalog, Problem};

fn unknown_outcome(step: &str, outcome: &str, declared: &[&str]) -> Problem {
    Problem::UnknownOutcome {
        step: step.to_owned(),
        outcome: outcome.to_owned(),
        declared: declared.iter().map(|name| (*name).to_owned()).collect(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn conditions_must_name_an_outcome_the_referenced_step_declares() {
    let operation = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["prepare"]
when = { step = "prepare", outcome = "passed" }
"#,
    )
    .await;
    let approval = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.gate]
kind = "approval"
artifact = "request"
rejection_artifact = "reason"

[steps.cycle]
kind = "loop"
body = ["gate"]
until = { step = "gate", outcome = "accepted" }
"#,
    )
    .await;
    let agent = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["write"]
when = { step = "write", outcome = "approved" }
"#,
    )
    .await;

    assert_eq!(
        operation,
        in_step("ship", unknown_outcome("prepare", "passed", &["done"]))
    );
    assert_eq!(
        approval,
        in_step(
            "cycle",
            unknown_outcome("gate", "accepted", &["approved", "rejected"])
        )
    );
    assert_eq!(
        agent,
        in_step("ship", unknown_outcome("write", "approved", &["done"]))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_missing_from_the_catalog_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.review]
kind = "agent"
agent = "reviewer"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "review",
            Problem::UnknownAgent {
                name: "reviewer".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_required_input_nobody_produces_before_the_step_is_refused() {
    let operation = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.check]
kind = "operation"
run = "checks.run"
"#,
    )
    .await;
    let renamed = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"
inputs = { request = "brief" }
"#,
    )
    .await;

    assert_eq!(
        operation,
        in_step(
            "check",
            Problem::MissingInput {
                input: "commit".to_owned()
            }
        )
    );
    assert_eq!(
        renamed,
        in_step(
            "write",
            Problem::MissingInput {
                input: "brief".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn renaming_an_input_the_agent_does_not_declare_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"
inputs = { brief = "request" }
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "write",
            Problem::UnknownAgentInput {
                input: "brief".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_approval_needs_its_artifact_produced_by_a_step_it_needs() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "approve",
            Problem::UnproducedArtifact {
                artifact: "draft".to_owned()
            }
        )
    );
}

const RENAMING_WORKFLOW: &str = r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"
inputs = { request = "base" }

[steps.check]
kind = "operation"
run = "checks.run"
inputs = []
"#;

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_takes_no_arguments_from_the_definition() {
    let invalid = workflow_refusal(RENAMING_WORKFLOW).await;

    assert_eq!(invalid.step.as_deref(), Some("check"));
    assert!(
        matches!(&invalid.problem, Problem::InvalidStep { detail } if detail.contains("inputs")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_renamed_input_is_satisfied_by_the_artifact_it_names() {
    let dir = ProjectDir::new();
    let catalog = Catalog::new()
        .workflow(
            "flow",
            r#"
schema = 1
name = "flow"

[steps.write]
kind = "agent"
agent = "writer"
inputs = { request = "base" }
"#,
        )
        .agent("writer", writer());
    let project = dir.project(catalog, default_config()).await;

    let created = project.create_run(new_run("flow")).await;

    assert!(created.is_ok(), "{created:?}");
}
