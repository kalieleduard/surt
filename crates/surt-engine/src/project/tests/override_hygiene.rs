use std::fs;

use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run, override_refusal};
use crate::project::{InvalidOverride, Problem};

async fn refused(dir: &ProjectDir) -> InvalidOverride {
    override_refusal(dir, draft_catalog(), "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_directory_that_is_not_a_built_in_agent_refuses_and_lists_the_known_ids() {
    let dir = ProjectDir::new();
    dir.write_local("writr", "agent.toml", "[profile]\nmodel = \"fake-model\"\n");

    let invalid = refused(&dir).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.problem),
        (
            ".surt/agents/writr",
            Problem::UnknownAgentDirectory {
                known: vec!["writer".to_owned()]
            }
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_beside_the_agent_directories_refuses_naming_it() {
    let dir = ProjectDir::new();
    fs::create_dir_all(dir.path().join(".surt/agents")).expect("the agents directory");
    fs::write(dir.path().join(".surt/agents/writer.toml"), "").expect("a stray file");

    let invalid = refused(&dir).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.problem),
        (".surt/agents/writer.toml", Problem::StrayAgentFile)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_inside_an_agent_directory_that_surt_does_not_read_refuses_naming_it() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[profile]\nmodel = \"fake-model\"\n",
    );
    dir.write_local("writer", "notes/instructions.md", "Write well.\n");

    let invalid = refused(&dir).await;

    assert_eq!(
        (invalid.file.as_str(), invalid.problem),
        (
            ".surt/agents/writer/notes/instructions.md",
            Problem::StrayAgentFile
        )
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn instructions_and_files_under_schemas_are_allowed_beside_agent_toml() {
    let dir = ProjectDir::new();
    dir.write_local(
        "writer",
        "agent.toml",
        "[profile]\nmodel = \"fake-model\"\n",
    );
    dir.write_local("writer", "instructions.md", "Write well.\n");
    dir.write_local("writer", "schemas/nested/extra.schema.json", "{}");

    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_agent_directories_freeze_the_built_in_agents_unchanged() {
    let dir = ProjectDir::new();
    fs::create_dir_all(dir.path().join(".surt/agents/writer/schemas")).expect("empty directories");
    let plain = ProjectDir::new();

    for project in [&dir, &plain] {
        let created = project
            .project(draft_catalog(), default_config())
            .await
            .create_run(new_run("draft"))
            .await
            .expect("the run is created");
        assert_eq!(created.local_agents, []);
    }

    assert_eq!(
        dir.frozen_agent("writer").await,
        plain.frozen_agent("writer").await
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn git_placeholder_files_anywhere_under_the_agents_directory_are_no_override() {
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

    for project in [&dir, &plain] {
        let created = project
            .project(draft_catalog(), default_config())
            .await
            .create_run(new_run("draft"))
            .await
            .expect("the run is created");
        assert_eq!(created.local_agents, []);
    }

    assert_eq!(
        dir.frozen_agent("writer").await,
        plain.frozen_agent("writer").await
    );
}
