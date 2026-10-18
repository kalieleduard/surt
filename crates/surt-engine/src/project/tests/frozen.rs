use super::fixtures::{ProjectDir, default_config, draft_catalog, new_run};
use crate::project::{AgentAssets, Catalog};

const INSTRUCTIONS_SHA256: &str =
    "8de2b45eaca7a1890d5d70979c1e1a085fa699d309cb1442f7f0db4e7b2fd9c1";

const REQUEST_SHA256: &str = "a08f988a903110d04e94bcf1248f310378288bc38f912cb64b01815025fea40e";

type RevisionRow = (String, i64, String, String, String, String, Vec<u8>);

async fn created(dir: &ProjectDir) {
    dir.project(draft_catalog(), default_config())
        .await
        .create_run(new_run("draft"))
        .await
        .expect("the run is created");
}

#[tokio::test(flavor = "multi_thread")]
async fn create_run_freezes_the_workflow_its_agents_and_the_config_as_snapshots() {
    let dir = ProjectDir::new();

    created(&dir).await;

    let frozen: Vec<(String, String, String, String)> = dir
        .rows("SELECT s.kind, s.name, f.rel_path, f.blob_sha256 FROM run_snapshot r JOIN snapshot s ON s.sha256 = r.snapshot_sha256 JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 ORDER BY s.kind, f.rel_path")
        .await;
    let files: Vec<(&str, &str, &str)> = frozen
        .iter()
        .map(|(kind, name, path, _)| (kind.as_str(), name.as_str(), path.as_str()))
        .collect();
    assert_eq!(
        files,
        [
            ("agent", "writer", "agent.toml"),
            ("agent", "writer", "instructions.md"),
            ("agent", "writer", "schemas/notes.schema.json"),
            ("config", "config", "config.toml"),
            ("workflow", "draft", "workflow.toml"),
        ]
    );
    assert_eq!(frozen[1].3, INSTRUCTIONS_SHA256);
    let pointers: Vec<(bool, bool)> = dir
        .rows("SELECT definition_sha256 = (SELECT sha256 FROM snapshot WHERE kind = 'workflow'), config_sha256 = (SELECT sha256 FROM snapshot WHERE kind = 'config') FROM workflow_run")
        .await;
    assert_eq!(pointers, [(true, true)]);
}

const READER_WORKFLOW: &str = r#"
schema = 1
name = "read"

[steps.read]
kind = "agent"
agent = "reader"
"#;

const READER: &str = r#"
schema = 1
id = "reader"

[inputs.base]
format = "json"
json_schema = "schemas/snapshot.schema.json"

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const READER_SCHEMAS: &[(&str, &str)] = &[
    (
        "schemas/snapshot.schema.json",
        r##"{"properties": {"base": {"$ref": "base.schema.json"}, "tip": {"$ref": "#/definitions/sha"}}}"##,
    ),
    (
        "schemas/base.schema.json",
        r##"{"properties": {"sha": {"$ref": "common/sha.schema.json#/definitions/sha"}}}"##,
    ),
    (
        "schemas/common/sha.schema.json",
        r#"{"definitions": {"sha": {"type": "string"}}}"#,
    ),
    ("schemas/unused.schema.json", r#"{"type": "object"}"#),
];

#[tokio::test(flavor = "multi_thread")]
async fn schemas_referenced_from_a_frozen_schema_are_frozen_with_it() {
    let dir = ProjectDir::new();
    let catalog = Catalog::new().workflow("read", READER_WORKFLOW).agent(
        "reader",
        AgentAssets {
            toml: READER,
            instructions: "Read.\n",
            schemas: READER_SCHEMAS,
        },
    );

    dir.project(catalog, default_config())
        .await
        .create_run(new_run("read"))
        .await
        .expect("the run is created");

    let frozen: Vec<(String,)> = dir
        .rows("SELECT f.rel_path FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 WHERE s.kind = 'agent' ORDER BY f.rel_path")
        .await;
    let paths: Vec<&str> = frozen.iter().map(|(path,)| path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "agent.toml",
            "instructions.md",
            "schemas/base.schema.json",
            "schemas/common/sha.schema.json",
            "schemas/snapshot.schema.json",
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_frozen_workflow_is_its_canonical_form_in_declaration_order() {
    let dir = ProjectDir::new();

    created(&dir).await;

    let stored: Vec<(Vec<u8>,)> = dir
        .rows("SELECT b.bytes FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE s.kind = 'workflow'")
        .await;
    let text = String::from_utf8(stored[0].0.clone()).expect("the definition is UTF-8");
    let reparsed: toml::Table = toml::from_str(&text).expect("the canonical form is TOML");
    assert_eq!(reparsed["cleanup"]["on_success"].as_str(), Some("clean"));
    let steps: Vec<&String> = reparsed["steps"]
        .as_table()
        .expect("a steps table")
        .keys()
        .collect();
    assert_eq!(steps, ["prepare", "write", "approve", "drafting"]);
    assert_eq!(
        reparsed["steps"]["approve"]["kind"].as_str(),
        Some("approval")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn initial_artifacts_become_human_first_revisions_of_the_root_run() {
    let dir = ProjectDir::new();

    created(&dir).await;

    let revisions: Vec<RevisionRow> = dir
        .rows("SELECT a.name, r.number, r.origin, r.human_action, r.actor, f.rel_path, b.bytes FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id JOIN revision_file f ON f.revision_id = r.id JOIN blob b ON b.sha256 = f.blob_sha256 ORDER BY a.name")
        .await;
    let human = |name: &str, path: &str, bytes: &[u8]| -> RevisionRow {
        (
            name.to_owned(),
            1,
            "human".to_owned(),
            "initial".to_owned(),
            "dev@example.com".to_owned(),
            path.to_owned(),
            bytes.to_vec(),
        )
    };
    assert_eq!(
        revisions,
        [
            human("base", "base.json", br#"{"sha": "abc"}"#),
            human("request", "request.txt", b"add login"),
        ]
    );
    let blobs: Vec<(String,)> = dir
        .rows("SELECT blob_sha256 FROM revision_file WHERE rel_path = 'request.txt'")
        .await;
    assert_eq!(blobs, [(REQUEST_SHA256.to_owned(),)]);
}
