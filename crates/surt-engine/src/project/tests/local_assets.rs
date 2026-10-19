use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run, override_refusal};
use crate::project::{LocalFiles, Problem};

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

async fn refused_reference(reference: &str) -> crate::project::InvalidOverride {
    let dir = ProjectDir::new();
    let schema = format!(r#"{{"properties": {{"status": {{"$ref": "{reference}"}}}}}}"#);
    dir.write_local("writer", NOTES, &schema);
    override_refusal(&dir, draft_catalog(), "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_network_ref_in_a_local_schema_refuses_naming_the_schema() {
    let reference = "https://example.com/status.schema.json";

    let invalid = refused_reference(reference).await;

    assert_eq!(
        invalid.file,
        ".surt/agents/writer/schemas/notes.schema.json"
    );
    assert_eq!(
        invalid.problem,
        Problem::NonLocalReference {
            reference: reference.to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn absolute_and_escaping_refs_in_a_local_schema_refuse() {
    for reference in ["/schemas/status.schema.json", "../../status.schema.json#/x"] {
        let invalid = refused_reference(reference).await;

        assert_eq!(
            invalid.problem,
            Problem::NonLocalReference {
                reference: reference.to_owned()
            }
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_ref_found_neither_locally_nor_built_in_refuses() {
    let invalid = refused_reference("defs/status.schema.json#/$defs/status").await;

    assert_eq!(
        invalid.file,
        ".surt/agents/writer/schemas/notes.schema.json"
    );
    assert_eq!(
        invalid.problem,
        Problem::MissingAsset {
            path: STATUS.to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn create_run_reports_instructions_and_schemas_next_to_agent_toml() {
    let dir = ProjectDir::new();
    dir.write_local("writer", "agent.toml", SUMMARY_OUTPUT);
    dir.write_local("writer", "instructions.md", LOCAL_INSTRUCTIONS);
    dir.write_local("writer", SUMMARY, STRICT_NOTES);
    dir.write_local("writer", STATUS, STATUS_SCHEMA);

    let created = dir
        .project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");

    assert_eq!(
        created.local_agents,
        [LocalFiles {
            agent: "writer".to_owned(),
            files: vec![
                "agent.toml".to_owned(),
                "instructions.md".to_owned(),
                STATUS.to_owned(),
                SUMMARY.to_owned(),
            ],
        }]
    );
}
