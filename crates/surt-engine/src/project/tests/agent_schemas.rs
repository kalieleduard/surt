use super::fixtures::{ProjectDir, default_config, draft_catalog, invalid_file};
use crate::project::{InvalidDefinition, Problem};

const NOTES: &str = "schemas/notes.schema.json";

const NOTES_FILE: &str = ".surt/agents/writer/schemas/notes.schema.json";

const STATUS: &str = "schemas/defs/status.schema.json";

async fn refused_schema(text: &str) -> InvalidDefinition {
    let dir = ProjectDir::new();
    dir.project(draft_catalog(), default_config()).await;
    dir.write_agent_file("writer", NOTES, text);
    invalid_file(&dir, "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_that_is_not_json_refuses_naming_the_schema() {
    let invalid = refused_schema("{ not json").await;

    assert_eq!(invalid.file.path(), NOTES_FILE);
    assert!(
        matches!(invalid.problem, Problem::InvalidSchema { .. }),
        "got {:?}",
        invalid.problem
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_schema_that_does_not_compile_refuses_naming_the_schema() {
    let invalid = refused_schema(r#"{"type": "no-such-type"}"#).await;

    assert_eq!(invalid.file.path(), NOTES_FILE);
    assert!(
        matches!(invalid.problem, Problem::InvalidSchema { .. }),
        "got {:?}",
        invalid.problem
    );
}

async fn refused_reference(reference: &str) -> InvalidDefinition {
    let schema = format!(r#"{{"properties": {{"status": {{"$ref": "{reference}"}}}}}}"#);
    refused_schema(&schema).await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_network_ref_in_a_schema_refuses_naming_the_schema() {
    let reference = "https://example.com/status.schema.json";

    let invalid = refused_reference(reference).await;

    assert_eq!(invalid.file.path(), NOTES_FILE);
    assert_eq!(
        invalid.problem,
        Problem::NonLocalReference {
            reference: reference.to_owned()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn absolute_and_escaping_refs_in_a_schema_refuse() {
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
async fn a_local_ref_to_a_file_the_agent_does_not_have_refuses() {
    let invalid = refused_reference("defs/status.schema.json#/$defs/status").await;

    assert_eq!(invalid.file.path(), NOTES_FILE);
    assert_eq!(
        invalid.problem,
        Problem::MissingAsset {
            path: STATUS.to_owned()
        }
    );
}
