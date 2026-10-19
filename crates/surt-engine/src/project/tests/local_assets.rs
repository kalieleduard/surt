use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run, override_refusal};
use crate::project::Problem;

const LOCAL_INSTRUCTIONS: &str = "Follow the project's style guide.\r\n";

const SUMMARY_OUTPUT: &str = r#"
[outputs.summary]
format = "json"
json_schema = "schemas/summary.schema.json"
"#;

const SUMMARY: &str = "schemas/summary.schema.json";

const NOTES: &str = "schemas/notes.schema.json";

const STRICT_NOTES: &str = r#"{"type": "object", "required": ["status"]}"#;

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

#[tokio::test(flavor = "multi_thread")]
async fn a_local_schema_overlays_the_built_in_schema_of_the_same_path() {
    let dir = ProjectDir::new();
    dir.write_local("writer", NOTES, STRICT_NOTES);

    create(&dir).await;

    assert_eq!(
        dir.frozen_agent_file("writer", NOTES).await.as_deref(),
        Some(STRICT_NOTES)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_named_by_the_local_agent_toml_resolves_to_the_local_file() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", SUMMARY_OUTPUT);
    dir.write_local("writer", SUMMARY, STRICT_NOTES);

    create(&dir).await;

    assert_eq!(
        dir.frozen_agent_file("writer", SUMMARY).await.as_deref(),
        Some(STRICT_NOTES)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_found_neither_locally_nor_built_in_refuses() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", SUMMARY_OUTPUT);

    let invalid = override_refusal(&dir, draft_catalog(), "draft").await;

    assert_eq!(invalid.file, ".surt/agents/writer/agent.toml");
    assert_eq!(
        invalid.problem,
        Problem::MissingAsset {
            path: SUMMARY.to_owned()
        }
    );
}

const SPLIT_NOTES: &str =
    r#"{"type": "object", "properties": {"status": {"$ref": "defs/status.schema.json"}}}"#;

const STATUS: &str = "schemas/defs/status.schema.json";

const STATUS_SCHEMA: &str = r#"{"enum": ["ok", "late"]}"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_local_schema_split_with_a_local_ref_freezes_its_closure() {
    let dir = ProjectDir::new();
    dir.write_local("writer", NOTES, SPLIT_NOTES);
    dir.write_local("writer", STATUS, STATUS_SCHEMA);

    create(&dir).await;

    assert_eq!(
        dir.frozen_agent_file("writer", STATUS).await.as_deref(),
        Some(STATUS_SCHEMA)
    );
}
