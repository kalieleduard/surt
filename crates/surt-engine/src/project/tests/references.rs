use super::fixtures::{in_step, workflow_refusal};
use crate::project::{Problem, StepField};

fn unknown(field: StepField, name: &str) -> Problem {
    Problem::UnknownStep {
        field,
        name: name.to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dangling_needs_is_refused_naming_the_step_and_the_reference() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["build"]
"#,
    )
    .await;

    assert_eq!(invalid, in_step("ship", unknown(StepField::Needs, "build")));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dangling_when_body_or_until_is_refused() {
    let when = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
when = { step = "check", outcome = "passed" }
"#,
    )
    .await;
    let body = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.cycle]
kind = "loop"
body = ["build"]
until = { step = "build", outcome = "done" }
"#,
    )
    .await;
    let until = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.cycle]
kind = "loop"
body = ["prepare"]
until = { step = "integrate", outcome = "merged" }
"#,
    )
    .await;

    assert_eq!(when, in_step("ship", unknown(StepField::When, "check")));
    assert_eq!(body, in_step("cycle", unknown(StepField::Body, "build")));
    assert_eq!(
        until,
        in_step("cycle", unknown(StepField::Until, "integrate"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cycle_in_needs_is_refused_with_its_path() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"
needs = ["ship"]

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["prepare"]
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "prepare",
            Problem::Cycle {
                path: vec![
                    "prepare".to_owned(),
                    "ship".to_owned(),
                    "prepare".to_owned()
                ]
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_outside_a_loop_cannot_need_a_step_of_its_body() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.cycle]
kind = "loop"
body = ["prepare"]
until = { step = "prepare", outcome = "done" }

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"
needs = ["prepare"]
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "ship",
            Problem::OutOfScope {
                field: StepField::Needs,
                name: "prepare".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_body_step_belongs_to_one_loop_and_loops_do_not_nest() {
    let shared = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.first]
kind = "loop"
body = ["prepare"]
until = { step = "prepare", outcome = "done" }

[steps.second]
kind = "loop"
body = ["prepare"]
until = { step = "prepare", outcome = "done" }
"#,
    )
    .await;
    let nested = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.inner]
kind = "loop"
body = ["prepare"]
until = { step = "prepare", outcome = "done" }

[steps.outer]
kind = "loop"
body = ["inner"]
until = { step = "inner", outcome = "done" }
"#,
    )
    .await;

    assert_eq!(
        shared,
        in_step(
            "second",
            Problem::SharedBodyStep {
                name: "prepare".to_owned()
            }
        )
    );
    assert_eq!(
        nested,
        in_step(
            "outer",
            Problem::NestedLoop {
                name: "inner".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn until_must_name_a_step_of_the_loop_body() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.ship]
kind = "operation"
run = "git.snapshot_run_branch"

[steps.cycle]
kind = "loop"
body = ["ship"]
until = { step = "prepare", outcome = "done" }
needs = ["prepare"]
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "cycle",
            Problem::UntilOutsideBody {
                name: "prepare".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_missing_from_the_registry_is_refused() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.push]
kind = "operation"
run = "git.push"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "push",
            Problem::UnknownOperation {
                name: "git.push".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn each_approval_gate_needs_its_own_rejection_artifact() {
    let invalid = workflow_refusal(
        r#"
schema = 1
name = "flow"

[steps.first]
kind = "approval"
artifact = "request"
rejection_artifact = "reason"

[steps.second]
kind = "approval"
artifact = "request"
rejection_artifact = "reason"
"#,
    )
    .await;

    assert_eq!(
        invalid,
        in_step(
            "second",
            Problem::DuplicateRejectionArtifact {
                artifact: "reason".to_owned()
            }
        )
    );
}
