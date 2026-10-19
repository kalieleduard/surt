use super::fixtures::{ProjectDir, draft_catalog, override_refusal};
use crate::project::{InvalidOverride, Problem};

const NOTES: &str = "schemas/notes.schema.json";

const NOTES_FILE: &str = ".surt/agents/writer/schemas/notes.schema.json";

const STATUS: &str = "schemas/defs/status.schema.json";

async fn refused_schema(text: &str) -> InvalidOverride {
    let dir = ProjectDir::new();
    dir.write_local("writer", NOTES, text);
    override_refusal(&dir, draft_catalog(), "draft").await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_schema_that_is_not_json_refuses_naming_the_schema() {
    let invalid = refused_schema("{ not json").await;

    assert_eq!(invalid.file, NOTES_FILE);
    assert!(
        matches!(invalid.problem, Problem::InvalidSchema { .. }),
        "got {:?}",
        invalid.problem
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_schema_that_does_not_compile_refuses_naming_the_schema() {
    let invalid = refused_schema(r#"{"type": "no-such-type"}"#).await;

    assert_eq!(invalid.file, NOTES_FILE);
    assert!(
        matches!(invalid.problem, Problem::InvalidSchema { .. }),
        "got {:?}",
        invalid.problem
    );
}

async fn refused_reference(reference: &str) -> InvalidOverride {
    let schema = format!(r#"{{"properties": {{"status": {{"$ref": "{reference}"}}}}}}"#);
    refused_schema(&schema).await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_network_ref_in_a_local_schema_refuses_naming_the_schema() {
    let reference = "https://example.com/status.schema.json";

    let invalid = refused_reference(reference).await;

    assert_eq!(invalid.file, NOTES_FILE);
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

    assert_eq!(invalid.file, NOTES_FILE);
    assert_eq!(
        invalid.problem,
        Problem::MissingAsset {
            path: STATUS.to_owned()
        }
    );
}
