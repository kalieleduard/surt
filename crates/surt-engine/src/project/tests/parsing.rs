use super::fixtures::{in_step, refusal, workflow_refusal};
use crate::project::{Catalog, DefinitionFile, InvalidDefinition, Problem};

fn at_top(problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Workflow("flow".to_owned()),
        step: None,
        problem,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_field_in_a_step_is_refused_naming_the_step_and_the_field() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.build]
kind = "agent"
agnet = "writer"
"#,
    )
    .await;

    assert_eq!(invalid.step.as_deref(), Some("build"));
    assert!(
        matches!(&invalid.problem, Problem::InvalidStep { detail } if detail.contains("agnet")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_kind_is_refused_naming_the_step_and_the_kind() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.lint]
kind = "script"
run = "cargo clippy"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "lint",
            Problem::UnknownKind {
                kind: "script".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_without_a_kind_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
run = "git.create_run_branch"
"#,
    )
    .await;

    assert_eq!(invalid, in_step("prepare", Problem::MissingKind));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_condition_is_only_an_outcome_comparison() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.again]
kind = "operation"
run = "git.create_run_branch"
needs = ["prepare"]
when = { step = "prepare", outcome = "done", expression = "outcome != 'done'" }
"#,
    )
    .await;

    assert_eq!(invalid.step.as_deref(), Some("again"));
    assert!(
        matches!(&invalid.problem, Problem::InvalidStep { detail } if detail.contains("expression")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_top_level_field_is_refused_with_its_line() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"
version = "2"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
"#,
    )
    .await;

    assert!(
        matches!(&invalid.problem, Problem::Syntax { line: Some(4), detail } if detail.contains("version")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_schema_version_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 2
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
"#,
    )
    .await;

    assert_eq!(invalid, at_top(Problem::UnsupportedSchema { found: 2 }));
}

#[tokio::test(flavor = "multi_thread")]
async fn cleanup_on_success_accepts_only_clean_or_keep() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[cleanup]
on_success = "keeep"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
"#,
    )
    .await;

    assert!(
        matches!(&invalid.problem, Problem::Syntax { line: Some(6), detail } if detail.contains("keep")),
        "{invalid:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_definition_whose_name_differs_from_its_catalog_entry_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "other"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        at_top(Problem::NameMismatch {
            found: "other".to_owned()
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_workflow_missing_from_the_catalog_is_refused() {
    let invalid = refusal(Catalog::new()).await;

    assert_eq!(
        invalid,
        at_top(Problem::UnknownWorkflow {
            name: "flow".to_owned()
        })
    );
}
