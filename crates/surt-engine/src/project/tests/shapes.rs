use super::fixtures::Catalog;
use super::fixtures::{ProjectDir, agent, default_config, input};
use crate::project::{Carry, NewRun};

const CYCLE: &str = r#"
schema = 1
name = "cycle"

[inputs.item]
format = "json"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"

[steps.check]
kind = "operation"
run = "checks.run"
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["check"]
when = { step = "check", outcome = "passed" }

[steps.integrate]
kind = "operation"
run = "git.integrate"
needs = ["review"]
when = { step = "review", outcome = "approved" }

[steps.loop]
kind = "loop"
body = ["build", "check", "review", "integrate"]
until = { step = "integrate", outcome = "merged" }
needs = ["setup"]
"#;

const BUILDER: &str = r#"
schema = 1
id = "builder"

[inputs.item]
format = "json"

[inputs.feedback]
format = "json"
optional = true

[inputs.check-report]
format = "json"
optional = true

[outputs.commit]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const REVIEWER: &str = r#"
schema = 1
id = "reviewer"
outcomes = ["approved", "changes_requested"]

[inputs.item]
format = "json"

[inputs.commit]
format = "json"

[inputs.check-report]
format = "json"

[outputs.feedback]
format = "json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_build_check_review_integrate_loop_is_accepted_and_only_its_loop_starts_pending() {
    let dir = ProjectDir::new();
    let catalog = Catalog::new()
        .workflow("cycle", CYCLE)
        .agent("builder", agent(BUILDER))
        .agent("reviewer", agent(REVIEWER));
    let project = dir.project(catalog, default_config()).await;

    project
        .create_run(NewRun {
            subject: "20261017-task-a".to_owned(),
            workflow: "cycle".to_owned(),
            inputs: vec![input("item", r#"{"key": "A"}"#)],
            carry: Carry::Nothing,
            merge_into_base: None,
            actor: "dev@example.com".to_owned(),
        })
        .await
        .expect("the run is created");

    let steps: Vec<(String, String)> = dir
        .rows("SELECT step_key, kind FROM step_run ORDER BY step_key")
        .await;
    let expected = [("loop", "loop"), ("setup", "operation")]
        .map(|(key, kind)| (key.to_owned(), kind.to_owned()));
    assert_eq!(steps, expected);
}
