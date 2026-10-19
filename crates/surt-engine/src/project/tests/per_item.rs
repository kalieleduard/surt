use super::fixtures::Catalog;
use super::fixtures::{ProjectDir, agent, default_config, in_step, new_run, refusal, writer};
use crate::project::{DefinitionFile, InvalidDefinition, Problem};

const PLANNER_AGENT: &str = r#"
schema = 1
id = "planner"

[inputs.request]
format = "text"

[outputs.plan]
format = "item_list"

[outputs.plan.item_files.task]
format = "markdown"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const PARENT: &str = r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.plan]
kind = "agent"
agent = "planner"

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "child"
pass = ["request"]
needs = ["plan"]
"#;

const CHILD: &str = r#"
schema = 1
name = "child"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.write]
kind = "agent"
agent = "writer"
needs = ["setup"]
"#;

pub(super) fn catalog(parent: &'static str, child: &'static str) -> Catalog {
    Catalog::new()
        .workflow("flow", parent)
        .workflow("child", child)
        .agent("planner", agent(PLANNER_AGENT))
        .agent("writer", writer())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_root_run_freezes_every_workflow_and_agent_reachable_through_per_item_steps() {
    let dir = ProjectDir::new();
    let project = dir.project(catalog(PARENT, CHILD), default_config()).await;

    project
        .create_run(new_run("flow"))
        .await
        .expect("the run is created");

    let frozen: Vec<(String, String)> = dir
        .rows("SELECT s.kind, s.name FROM run_snapshot r JOIN snapshot s ON s.sha256 = r.snapshot_sha256 ORDER BY s.kind, s.name")
        .await;
    let expected = [
        ("agent", "planner"),
        ("agent", "writer"),
        ("config", "config"),
        ("workflow", "child"),
        ("workflow", "flow"),
    ]
    .map(|(kind, name)| (kind.to_owned(), name.to_owned()));
    assert_eq!(frozen, expected);
    let steps: Vec<(String,)> = dir
        .rows("SELECT step_key FROM step_run ORDER BY step_key")
        .await;
    assert_eq!(steps, [("implement".to_owned(),), ("plan".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_reads_only_its_item_and_what_the_parent_passes() {
    let invalid = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.plan]
kind = "agent"
agent = "planner"

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "child"
needs = ["plan"]
"#,
        CHILD,
    ))
    .await;

    assert_eq!(
        invalid,
        InvalidDefinition {
            file: DefinitionFile::Workflow("child".to_owned()),
            field: None,
            step: Some("write".to_owned()),
            problem: Problem::MissingInput {
                input: "request".to_owned()
            },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn items_must_be_an_item_list_produced_before_the_step() {
    let initial = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.implement]
kind = "per_item"
items = "request"
workflow = "child"
"#,
        CHILD,
    ))
    .await;
    let unproduced = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "child"
"#,
        CHILD,
    ))
    .await;

    assert_eq!(
        initial,
        in_step(
            "implement",
            Problem::NotAnItemList {
                artifact: "request".to_owned()
            }
        )
    );
    assert_eq!(
        unproduced,
        in_step(
            "implement",
            Problem::UnproducedArtifact {
                artifact: "plan".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pass_names_an_artifact_the_parent_has_before_the_step() {
    let invalid = refusal(catalog(
        r#"
schema = 1
name = "flow"

[inputs.request]
format = "text"

[steps.plan]
kind = "agent"
agent = "planner"

[steps.implement]
kind = "per_item"
items = "plan"
workflow = "child"
pass = ["spec"]
needs = ["plan"]
"#,
        CHILD,
    ))
    .await;

    assert_eq!(
        invalid,
        in_step(
            "implement",
            Problem::UnproducedArtifact {
                artifact: "spec".to_owned()
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_per_item_workflow_must_exist_and_must_not_reach_itself() {
    let unknown = refusal(
        Catalog::new()
            .workflow("flow", PARENT)
            .agent("planner", agent(PLANNER_AGENT)),
    )
    .await;
    let recursive = refusal(catalog(
        PARENT,
        r#"
schema = 1
name = "child"

[steps.plan]
kind = "agent"
agent = "planner"
inputs = { request = "item" }

[steps.again]
kind = "per_item"
items = "plan"
workflow = "child"
needs = ["plan"]
"#,
    ))
    .await;

    assert_eq!(
        unknown,
        in_step(
            "implement",
            Problem::UnknownWorkflow {
                name: "child".to_owned()
            }
        )
    );
    assert_eq!(
        recursive,
        InvalidDefinition {
            file: DefinitionFile::Workflow("child".to_owned()),
            field: None,
            step: Some("again".to_owned()),
            problem: Problem::RecursiveWorkflow {
                name: "child".to_owned()
            },
        }
    );
}
