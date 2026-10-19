use super::fixtures::{
    ProjectDir, WRITER_AGENT, default_config, draft_catalog, invalid_file, new_run,
};
use crate::project::{DefinitionFile, Problem};

const CRLF_INSTRUCTIONS: &str = "Follow the project's style guide.\r\n";

const SUMMARY_OUTPUT: &str =
    "\n[outputs.summary]\nformat = \"json\"\njson_schema = \"schemas/summary.schema.json\"\n";

const SUMMARY: &str = "schemas/summary.schema.json";

const NOTES: &str = "schemas/notes.schema.json";

const STRICT_NOTES: &str = r#"{"type": "object", "required": ["status"]}"#;

const SPLIT_NOTES: &str =
    r#"{"type": "object", "properties": {"status": {"$ref": "defs/status.schema.json"}}}"#;

const STATUS: &str = "schemas/defs/status.schema.json";

const STATUS_SCHEMA: &str = r#"{"enum": ["ok", "late"]}"#;

fn with_summary() -> String {
    WRITER_AGENT.replace("\n[profile]", &format!("{SUMMARY_OUTPUT}\n[profile]"))
}

async fn created(files: &[(&str, &str)]) -> ProjectDir {
    let dir = ProjectDir::new();
    let project = dir.project(draft_catalog(), default_config()).await;
    for (rel_path, text) in files {
        dir.write_agent_file("writer", rel_path, text);
    }
    project
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
    dir
}

#[tokio::test(flavor = "multi_thread")]
async fn instructions_are_frozen_byte_for_byte_as_read() {
    let dir = created(&[("instructions.md", CRLF_INSTRUCTIONS)]).await;

    assert_eq!(
        dir.frozen_agent_file("writer", "instructions.md")
            .await
            .as_deref(),
        Some(CRLF_INSTRUCTIONS)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_named_by_agent_toml_is_frozen_from_the_agents_schemas_directory() {
    let summary = with_summary();
    let dir = created(&[
        ("agent.toml", &summary),
        (SUMMARY, STRICT_NOTES),
        (NOTES, STRICT_NOTES),
    ])
    .await;

    for path in [SUMMARY, NOTES] {
        assert_eq!(
            dir.frozen_agent_file("writer", path).await.as_deref(),
            Some(STRICT_NOTES),
            "{path}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_split_with_a_local_ref_freezes_its_closure() {
    let dir = created(&[(NOTES, SPLIT_NOTES), (STATUS, STATUS_SCHEMA)]).await;

    assert_eq!(
        dir.frozen_agent_file("writer", STATUS).await.as_deref(),
        Some(STATUS_SCHEMA)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_named_by_agent_toml_but_absent_from_schemas_refuses() {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    dir.write_agent_file("writer", "agent.toml", &with_summary());

    let invalid = invalid_file(&dir, "draft").await;

    assert_eq!(invalid.file, DefinitionFile::Agent("writer".to_owned()));
    assert_eq!(
        invalid.problem,
        Problem::MissingAsset {
            path: SUMMARY.to_owned()
        }
    );
}
