use super::fixtures::Catalog;
use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use super::per_item::catalog;
use crate::project::{CleanupConfig, Config, OnSuccess};

const KEEPING_PARENT: &str = r#"
schema = 1
name = "flow"

[cleanup]
on_success = "keep"

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

const CLEANING_PARENT: &str = r#"
schema = 1
name = "flow"

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

const KEEPING_CHILD: &str = r#"
schema = 1
name = "child"

[cleanup]
on_success = "keep"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"
"#;

const CLEANING_CHILD: &str = r#"
schema = 1
name = "child"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"
"#;

fn configured(on_success: Option<OnSuccess>) -> Config {
    Config {
        cleanup: CleanupConfig { on_success },
        ..default_config()
    }
}

async fn frozen(catalog: Catalog, workflow: &str, config: Config) -> (ProjectDir, toml::Table) {
    let dir = ProjectDir::new();
    dir.project(catalog, config)
        .await
        .create_run(new_run(workflow))
        .await
        .expect("the run is created");
    let stored: Vec<(Vec<u8>,)> = dir
        .rows("SELECT b.bytes FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE s.kind = 'config'")
        .await;
    let text = String::from_utf8(stored[0].0.clone()).expect("the config is UTF-8");
    (dir, toml::from_str(&text).expect("the config is TOML"))
}

fn on_success(config: &toml::Table) -> Option<&str> {
    config["cleanup"]["on_success"].as_str()
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_configured_policy_the_root_definition_decides() {
    let (_, cleaned) = frozen(draft_catalog(), "draft", configured(None)).await;
    let (dir, kept) = frozen(
        catalog(KEEPING_PARENT, CLEANING_CHILD),
        "flow",
        configured(None),
    )
    .await;

    assert_eq!(on_success(&cleaned), Some("clean"));
    assert_eq!(on_success(&kept), Some("keep"));
    let definition: Vec<(Vec<u8>,)> = dir
        .rows("SELECT b.bytes FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE s.kind = 'workflow' AND s.name = 'flow'")
        .await;
    let canonical: toml::Table =
        toml::from_str(&String::from_utf8_lossy(&definition[0].0)).expect("TOML");
    assert_eq!(canonical["cleanup"]["on_success"].as_str(), Some("keep"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_configured_policy_wins_over_the_root_definition() {
    let (_, kept) = frozen(draft_catalog(), "draft", configured(Some(OnSuccess::Keep))).await;
    let (_, cleaned) = frozen(
        catalog(KEEPING_PARENT, CLEANING_CHILD),
        "flow",
        configured(Some(OnSuccess::Clean)),
    )
    .await;

    assert_eq!(on_success(&kept), Some("keep"));
    assert_eq!(on_success(&cleaned), Some("clean"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_definition_does_not_choose_the_policy() {
    let (_, frozen) = frozen(
        catalog(CLEANING_PARENT, KEEPING_CHILD),
        "flow",
        configured(None),
    )
    .await;

    assert_eq!(on_success(&frozen), Some("clean"));
}
