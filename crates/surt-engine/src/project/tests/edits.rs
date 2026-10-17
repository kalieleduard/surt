use super::fixtures::{
    ProjectDir, WRITER_AGENT, WRITER_SCHEMAS, default_config, draft_catalog, new_run,
};
use crate::project::{AgentAssets, Catalog, NewRun};

const EDITED_WORKFLOW: &str = r#"
schema = 1
name = "draft"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.write]
kind = "agent"
agent = "writer"
needs = ["prepare"]
"#;

const EDITED_INSTRUCTIONS: &str = "Write a shorter draft.\n";

type FrozenFile = (String, String, Vec<u8>);

const FROZEN_FILES_OF_FIRST_RUN: &str = "SELECT s.kind, f.rel_path, b.bytes FROM workflow_run w JOIN run_snapshot r ON r.root_run_id = w.id JOIN snapshot s ON s.sha256 = r.snapshot_sha256 JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE w.ordinal = 1 AND w.subject_id = (SELECT id FROM subject WHERE name = '20261017-add-login') ORDER BY s.kind, f.rel_path";

fn edited_catalog() -> Catalog {
    Catalog::new().workflow("draft", EDITED_WORKFLOW).agent(
        "writer",
        AgentAssets {
            toml: WRITER_AGENT,
            instructions: EDITED_INSTRUCTIONS,
            schemas: WRITER_SCHEMAS,
        },
    )
}

fn other_subject() -> NewRun {
    NewRun {
        subject: "20261017-add-logout".to_owned(),
        ..new_run("draft")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn editing_definitions_and_config_after_create_run_leaves_its_snapshots_unchanged() {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the first run is created");
    let before: Vec<FrozenFile> = dir.rows(FROZEN_FILES_OF_FIRST_RUN).await;
    let mut edited_config = default_config();
    edited_config.max_attempts = 7;

    dir.project(edited_catalog(), edited_config)
        .await
        .create_run(other_subject())
        .await
        .expect("a run with the edited definitions is created");

    assert_eq!(
        dir.rows::<FrozenFile>(FROZEN_FILES_OF_FIRST_RUN).await,
        before
    );
    let distinct: Vec<(i64, i64, i64)> = dir
        .rows("SELECT count(DISTINCT definition_sha256), count(DISTINCT config_sha256), (SELECT count(*) FROM snapshot WHERE kind = 'agent') FROM workflow_run")
        .await;
    assert_eq!(distinct, [(2, 2, 2)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn runs_freezing_identical_content_share_one_snapshot_of_each_kind() {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    project
        .create_run(new_run("draft"))
        .await
        .expect("the first run is created");

    project
        .create_run(other_subject())
        .await
        .expect("the second run is created");

    let counts: Vec<(i64, i64, i64)> = dir
        .rows("SELECT (SELECT count(*) FROM snapshot), (SELECT count(*) FROM run_snapshot), (SELECT count(DISTINCT definition_sha256) FROM workflow_run)")
        .await;
    assert_eq!(counts, [(3, 6, 1)]);
}
