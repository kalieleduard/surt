use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};

const LOCAL_INSTRUCTIONS: &str = "Follow the project's style guide.\r\n";

async fn create(dir: &ProjectDir) {
    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_instructions_md_replaces_the_built_in_text_and_is_frozen_as_read() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "instructions.md", LOCAL_INSTRUCTIONS);

    create(&dir).await;

    assert_eq!(
        dir.frozen_agent_file("writer", "instructions.md")
            .await
            .as_deref(),
        Some(LOCAL_INSTRUCTIONS)
    );
}
