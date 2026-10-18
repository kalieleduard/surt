use sqlx::{FromRow, SqliteConnection};

use super::runs::new_id;
use super::{Store, StoreError};
use crate::artifacts::{identity, sha256_hex};

#[derive(Clone, Copy)]
pub(super) enum Origin<'a> {
    Execution(&'a str),
    Human {
        action: &'static str,
        actor: &'a str,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Published {
    pub(crate) artifact_id: String,
    pub(crate) revision_id: String,
    pub(crate) number: u32,
    pub(crate) files: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub(crate) struct StoredRevision {
    pub(crate) number: u32,
    pub(crate) sha256: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
    pub(crate) producer: Producer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Producer {
    Step { key: String, iteration: Option<u32> },
    Human(String),
}

#[derive(FromRow)]
struct ProducerRow {
    number: i64,
    identity_sha256: String,
    human_action: Option<String>,
    step_key: Option<String>,
    iteration: Option<i64>,
}

impl Store {
    pub(crate) async fn revision(&self, id: &str) -> Result<StoredRevision, StoreError> {
        let row: ProducerRow = sqlx::query_as(
            "SELECT r.number, r.identity_sha256, r.human_action, coalesce(s.step_key, g.step_key) AS step_key, \
             coalesce(s.iteration, g.iteration) AS iteration FROM artifact_revision r \
             LEFT JOIN execution e ON e.id = r.execution_id \
             LEFT JOIN step_run s ON s.id = e.step_run_id \
             LEFT JOIN approval a ON a.reason_revision_id = r.id \
             LEFT JOIN step_run g ON g.id = a.gate_step_run_id WHERE r.id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        let files = sqlx::query_as(
            "SELECT f.rel_path, b.bytes FROM revision_file f \
             JOIN blob b ON b.sha256 = f.blob_sha256 WHERE f.revision_id = ? ORDER BY f.rel_path",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let producer = match (row.step_key, row.human_action) {
            (Some(key), _) => Producer::Step {
                key,
                iteration: row.iteration.map(number),
            },
            (None, action) => Producer::Human(action.unwrap_or_default()),
        };
        Ok(StoredRevision {
            number: number(row.number),
            sha256: row.identity_sha256,
            files,
            producer,
        })
    }

    pub(crate) async fn revision_paths(&self, id: &str) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT rel_path FROM revision_file WHERE revision_id = ? ORDER BY rel_path",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?)
    }
}

pub(super) fn number(value: i64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

pub(super) async fn artifact_id(
    connection: &mut SqliteConnection,
    run: &str,
    name: &str,
) -> Result<String, sqlx::Error> {
    sqlx::query(
        "INSERT INTO artifact (id, workflow_run_id, name) VALUES (?, ?, ?) \
         ON CONFLICT (workflow_run_id, name) DO NOTHING",
    )
    .bind(new_id())
    .bind(run)
    .bind(name)
    .execute(&mut *connection)
    .await?;
    sqlx::query_scalar("SELECT id FROM artifact WHERE workflow_run_id = ? AND name = ?")
        .bind(run)
        .bind(name)
        .fetch_one(connection)
        .await
}

pub(super) async fn publish(
    connection: &mut SqliteConnection,
    artifact_id: &str,
    files: &[(String, Vec<u8>)],
    origin: Origin<'_>,
    now: i64,
) -> Result<Published, sqlx::Error> {
    let hashed: Vec<(String, String)> = files
        .iter()
        .map(|(rel_path, bytes)| (rel_path.clone(), sha256_hex(bytes)))
        .collect();
    let identity_sha256 = identity(
        hashed
            .iter()
            .map(|(rel_path, sha256)| (rel_path.as_str(), sha256.as_str())),
    );
    let next: i64 = sqlx::query_scalar(
        "SELECT coalesce(max(number), 0) + 1 FROM artifact_revision WHERE artifact_id = ?",
    )
    .bind(artifact_id)
    .fetch_one(&mut *connection)
    .await?;
    let revision_id = new_id();
    let (execution, action, actor) = match origin {
        Origin::Execution(execution) => (Some(execution), None, None),
        Origin::Human { action, actor } => (None, Some(action), Some(actor)),
    };
    sqlx::query(
        "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, execution_id, human_action, actor, created_at) \
         VALUES (?, ?, ?, ?, CASE WHEN ? IS NULL THEN 'human' ELSE 'execution' END, ?, ?, ?, ?)",
    )
    .bind(&revision_id)
    .bind(artifact_id)
    .bind(next)
    .bind(&identity_sha256)
    .bind(execution)
    .bind(execution)
    .bind(action)
    .bind(actor)
    .bind(now)
    .execute(&mut *connection)
    .await?;
    for ((rel_path, sha256), (_, bytes)) in hashed.iter().zip(files) {
        insert_blob(&mut *connection, sha256, bytes).await?;
        sqlx::query(
            "INSERT INTO revision_file (revision_id, rel_path, blob_sha256) VALUES (?, ?, ?)",
        )
        .bind(&revision_id)
        .bind(rel_path)
        .bind(sha256)
        .execute(&mut *connection)
        .await?;
    }
    Ok(Published {
        artifact_id: artifact_id.to_owned(),
        revision_id,
        number: number(next),
        files: hashed,
    })
}

pub(super) async fn insert_blob(
    connection: &mut SqliteConnection,
    sha256: &str,
    bytes: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO blob (sha256, size, bytes) VALUES (?, ?, ?)")
        .bind(sha256)
        .bind(i64::try_from(bytes.len()).unwrap_or(i64::MAX))
        .bind(bytes)
        .execute(connection)
        .await?;
    Ok(())
}
