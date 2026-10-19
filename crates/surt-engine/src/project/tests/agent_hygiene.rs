use std::fs;

use super::fixtures::{ProjectDir, default_config, draft_catalog, invalid_file, new_run};
use crate::project::Problem;

async fn created(dir: &ProjectDir) {
    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_inside_a_used_agent_directory_that_surt_does_not_read_refuses_naming_it() {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    dir.write_agent_file("writer", "notes/instructions.md", "Write well.\n");

    let invalid = invalid_file(&dir, "draft").await;

    assert_eq!(
        (invalid.file.path(), invalid.problem),
        (
            ".surt/agents/writer/notes/instructions.md".to_owned(),
            Problem::StrayAgentFile
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn nested_files_under_schemas_are_allowed_beside_agent_toml() {
    let dir = ProjectDir::new();
    dir.write_agent_file("writer", "schemas/nested/extra.schema.json", "{}");

    created(&dir).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn git_placeholder_files_anywhere_under_an_agent_directory_are_ignored() {
    let dir = ProjectDir::new();
    for placeholder in [
        ".gitkeep",
        "writer/.keep",
        "writer/schemas/.gitkeep",
        "writer/drafts/.gitkeep",
    ] {
        let path = dir.path().join(".surt/agents").join(placeholder);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the directory");
        fs::write(path, "").expect("a placeholder");
    }
    let plain = ProjectDir::new();

    created(&dir).await;
    created(&plain).await;

    assert_eq!(
        dir.frozen_agent("writer").await,
        plain.frozen_agent("writer").await
    );
}
