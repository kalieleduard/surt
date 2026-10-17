use std::path::PathBuf;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{FromRow, Sqlite, SqlitePool};
use tempfile::TempDir;

use crate::project::{
    AgentAssets, ArtifactFile, Catalog, DefinitionFile, Error, InitialArtifact, InvalidDefinition,
    NewRun, Problem, Project, Refusal,
};
use crate::store::Store;
use crate::workflow::{GitConfig, MergeIntoBase, RunConfig};

pub(super) const DRAFT_WORKFLOW: &str = r#"
schema = 1
name = "draft"
description = "Write and approve a draft."

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
needs = ["write"]

[steps.drafting]
kind = "loop"
body = ["write", "approve"]
until = { step = "approve", outcome = "approved" }
needs = ["prepare"]
"#;

pub(super) const WRITER_AGENT: &str = r#"
schema = 1
id = "writer"

[inputs.request]
format = "text"

[inputs.draft]
format = "markdown"
optional = true

[inputs.draft-rejection]
format = "text"
optional = true

[outputs.draft]
format = "markdown"

[outputs.notes]
format = "json"
json_schema = "schemas/notes.schema.json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) const WRITER_INSTRUCTIONS: &str = "Write the draft the request asks for.\n";

pub(super) const NOTES_SCHEMA: &str = r#"{"type": "object"}"#;

pub(super) const WRITER_SCHEMAS: &[(&str, &str)] = &[("schemas/notes.schema.json", NOTES_SCHEMA)];

pub(super) const SUBJECT: &str = "20261017-add-login";

pub(super) fn writer() -> AgentAssets {
    AgentAssets {
        toml: WRITER_AGENT,
        instructions: WRITER_INSTRUCTIONS,
        schemas: WRITER_SCHEMAS,
    }
}

pub(super) fn agent(toml: &'static str) -> AgentAssets {
    AgentAssets {
        toml,
        instructions: "Do the work.\n",
        schemas: &[],
    }
}

pub(super) fn draft_catalog() -> Catalog {
    Catalog::new()
        .workflow("draft", DRAFT_WORKFLOW)
        .agent("writer", writer())
}

pub(super) fn default_config() -> RunConfig {
    RunConfig {
        max_attempts: 3,
        max_iterations: 5,
        git: GitConfig {
            merge_into_base: MergeIntoBase::Never,
        },
        checks: Vec::new(),
    }
}

pub(super) fn new_run(workflow: &str) -> NewRun {
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: workflow.to_owned(),
        initial_artifacts: vec![
            initial("request", "request.txt", b"add login"),
            initial("base", "base.json", br#"{"sha": "abc"}"#),
        ],
        actor: "dev@example.com".to_owned(),
    }
}

pub(super) fn initial(name: &str, rel_path: &str, bytes: &[u8]) -> InitialArtifact {
    InitialArtifact {
        name: name.to_owned(),
        files: vec![ArtifactFile {
            rel_path: rel_path.to_owned(),
            bytes: bytes.to_vec(),
        }],
    }
}

pub(super) async fn refusal(catalog: Catalog) -> InvalidDefinition {
    let dir = ProjectDir::new();
    let project = dir.project(catalog, default_config()).await;

    let result = project.create_run(new_run("flow")).await;

    let written: Vec<(i64, i64)> = dir
        .rows("SELECT (SELECT count(*) FROM subject), (SELECT count(*) FROM snapshot)")
        .await;
    assert_eq!(written, [(0, 0)]);
    match result {
        Err(Error::Refused(Refusal::InvalidDefinition(invalid))) => *invalid,
        other => panic!("expected an invalid definition, got {other:?}"),
    }
}

pub(super) async fn workflow_refusal(text: &'static str) -> InvalidDefinition {
    refusal(
        Catalog::new()
            .workflow("flow", text)
            .agent("writer", writer()),
    )
    .await
}

pub(super) fn in_step(step: &str, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Workflow("flow".to_owned()),
        step: Some(step.to_owned()),
        problem,
    }
}

pub(super) struct ProjectDir {
    root: TempDir,
}

impl ProjectDir {
    pub(super) fn new() -> Self {
        Self {
            root: TempDir::new().expect("a temporary directory"),
        }
    }

    fn database(&self) -> PathBuf {
        self.root.path().join("surt.db")
    }

    pub(super) async fn project(&self, catalog: Catalog, config: RunConfig) -> Project {
        let lock = self
            .root
            .path()
            .join("runtime")
            .join("locks")
            .join("db.lock");
        let store = Store::open(&self.database(), &lock)
            .await
            .expect("the store opens");
        Project {
            store,
            catalog,
            config,
        }
    }

    pub(super) async fn execute(&self, sql: &'static str) {
        let pool = SqlitePool::connect_with(SqliteConnectOptions::new().filename(self.database()))
            .await
            .expect("the database opens");
        sqlx::query(sql)
            .execute(&pool)
            .await
            .expect("the statement runs");
        pool.close().await;
    }

    pub(super) async fn rows<T>(&self, sql: &'static str) -> Vec<T>
    where
        T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let options = SqliteConnectOptions::new()
            .filename(self.database())
            .read_only(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .expect("the database opens read-only");
        let rows = sqlx::query_as::<Sqlite, T>(sql)
            .fetch_all(&pool)
            .await
            .expect("the rows are readable");
        pool.close().await;
        rows
    }
}
