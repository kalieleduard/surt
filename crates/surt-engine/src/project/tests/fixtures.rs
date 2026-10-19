mod catalog;
mod refusals;

use std::fs;
use std::path::PathBuf;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{FromRow, Sqlite, SqlitePool};
use tempfile::TempDir;

use crate::project::layout::Layout;
use crate::project::{Access, SurtHome, open_store};
use crate::project::{ArtifactFile, Config, GlobalConfig, InitialArtifact, NewRun, Project};
#[cfg(feature = "test-support")]
use crate::store::Faults;
pub(super) use catalog::{
    AgentAssets, Catalog, WRITER_AGENT, WRITER_SCHEMAS, agent, draft_catalog, writer,
};
pub(super) use refusals::{in_step, invalid_file, refusal, workflow_refusal};

pub(super) const SUBJECT: &str = "20261017-add-login";

pub(super) fn default_config() -> Config {
    Config::default()
}

pub(super) fn new_run(workflow: &str) -> NewRun {
    NewRun {
        subject: SUBJECT.to_owned(),
        workflow: workflow.to_owned(),
        initial_artifacts: vec![
            initial("request", "request.txt", b"add login"),
            initial("base", "base.json", br#"{"sha": "abc"}"#),
        ],
        carried: Vec::new(),
        merge_into_base: None,
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

pub(super) struct ProjectDir {
    dir: TempDir,
}

impl ProjectDir {
    pub(super) fn new() -> Self {
        let project = ProjectDir {
            dir: TempDir::new().expect("a temporary directory"),
        };
        fs::create_dir_all(project.path()).expect("the project directory");
        fs::create_dir_all(project.home()).expect("the home directory");
        project
    }

    pub(super) fn path(&self) -> PathBuf {
        dunce::canonicalize(self.dir.path())
            .expect("a canonical temporary directory")
            .join("project")
    }

    pub(super) fn home(&self) -> PathBuf {
        dunce::canonicalize(self.dir.path())
            .expect("a canonical temporary directory")
            .join("home")
    }

    fn layout(&self) -> Layout {
        Layout::new(self.path())
    }

    fn database(&self) -> PathBuf {
        self.layout().database()
    }

    pub(super) async fn project(&self, catalog: Catalog, config: Config) -> Project {
        catalog.write(&self.layout().dot_surt());
        open_store(&self.layout())
            .await
            .expect("the store opens")
            .close()
            .await;
        Project {
            layout: self.layout(),
            id: "test-project".to_owned(),
            config,
            global: GlobalConfig::default(),
            home: SurtHome::at(&self.home()),
            access: Access::ReadWrite,
            #[cfg(feature = "test-support")]
            faults: Faults::default(),
        }
    }

    pub(super) async fn execute(&self, sql: &str) {
        let pool = SqlitePool::connect_with(SqliteConnectOptions::new().filename(self.database()))
            .await
            .expect("the database opens");
        sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
            .execute(&pool)
            .await
            .expect("the statement runs");
        pool.close().await;
    }

    pub(super) async fn rows<T>(&self, sql: &str) -> Vec<T>
    where
        T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let options = SqliteConnectOptions::new()
            .filename(self.database())
            .read_only(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .expect("the database opens read-only");
        let rows = sqlx::query_as::<Sqlite, T>(sqlx::AssertSqlSafe(sql.to_owned()))
            .fetch_all(&pool)
            .await
            .expect("the rows are readable");
        pool.close().await;
        rows
    }

    pub(super) async fn assert_nothing_written(&self) {
        let written: Vec<(i64, i64)> = self
            .rows("SELECT (SELECT count(*) FROM subject), (SELECT count(*) FROM snapshot)")
            .await;
        assert_eq!(written, [(0, 0)]);
    }

    pub(super) async fn event_data(&self, kind: &str) -> Vec<serde_json::Value> {
        let sql = format!("SELECT data FROM event WHERE kind = '{kind}' ORDER BY seq");
        let rows: Vec<(String,)> = self.rows(&sql).await;
        rows.into_iter()
            .map(|(data,)| serde_json::from_str(&data).expect("event data is JSON"))
            .collect()
    }
}
