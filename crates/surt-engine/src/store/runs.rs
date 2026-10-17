use sqlx::SqliteConnection;
use uuid::Uuid;

use super::{Store, StoreError};
use crate::workflow::FrozenRun;

mod content;

use content::{insert_initial_artifact, insert_snapshot};

const RUN_CREATED: &str = "run.created";

pub(crate) struct NewRunRows {
    pub(crate) subject: String,
    pub(crate) frozen: FrozenRun,
    pub(crate) initial_artifacts: Vec<NewInitialArtifact>,
    pub(crate) actor: String,
    pub(crate) now: i64,
}

pub(crate) struct NewInitialArtifact {
    pub(crate) name: String,
    pub(crate) files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RunCreation {
    Created(String),
    ActiveRunExists(String),
}

impl Store {
    pub(crate) async fn create_run(&self, rows: NewRunRows) -> Result<RunCreation, StoreError> {
        let mut transaction = self.begin_write().await?;
        let subject = subject_id(&mut transaction, &rows.subject, rows.now).await?;
        if let Some(active) = active_root_run(&mut transaction, &subject).await? {
            return Ok(RunCreation::ActiveRunExists(active));
        }
        for snapshot in &rows.frozen.snapshots {
            insert_snapshot(&mut transaction, snapshot).await?;
        }
        let run = new_id();
        insert_root_run(&mut transaction, &run, &subject, &rows).await?;
        for artifact in &rows.initial_artifacts {
            insert_initial_artifact(&mut transaction, &run, artifact, &rows).await?;
        }
        for step in &rows.frozen.steps {
            sqlx::query("INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, created_at) VALUES (?, ?, ?, ?, 'pending', ?)")
                .bind(new_id())
                .bind(&run)
                .bind(&step.key)
                .bind(step.kind.as_str())
                .bind(rows.now)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query(
            "INSERT INTO event (at, root_run_id, workflow_run_id, kind) VALUES (?, ?, ?, ?)",
        )
        .bind(rows.now)
        .bind(&run)
        .bind(&run)
        .bind(RUN_CREATED)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(RunCreation::Created(run))
    }
}

pub(super) fn new_id() -> String {
    Uuid::now_v7().to_string()
}

async fn subject_id(
    connection: &mut SqliteConnection,
    name: &str,
    now: i64,
) -> Result<String, sqlx::Error> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM subject WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut *connection)
        .await?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = new_id();
    sqlx::query("INSERT INTO subject (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(now)
        .execute(&mut *connection)
        .await?;
    Ok(id)
}

async fn active_root_run(
    connection: &mut SqliteConnection,
    subject: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM workflow_run WHERE subject_id = ? AND parent_step_run_id IS NULL AND state = 'active'",
    )
    .bind(subject)
    .fetch_optional(connection)
    .await
}

async fn insert_root_run(
    connection: &mut SqliteConnection,
    run: &str,
    subject: &str,
    rows: &NewRunRows,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO workflow_run (id, subject_id, root_run_id, definition_sha256, ordinal, config_sha256, state, created_at) \
         VALUES (?1, ?2, ?1, ?3, (SELECT coalesce(max(ordinal), 0) + 1 FROM workflow_run WHERE subject_id = ?2), ?4, 'active', ?5)",
    )
    .bind(run)
    .bind(subject)
    .bind(&rows.frozen.definition_sha256)
    .bind(&rows.frozen.config_sha256)
    .bind(rows.now)
    .execute(&mut *connection)
    .await?;
    for snapshot in &rows.frozen.snapshots {
        sqlx::query(
            "INSERT OR IGNORE INTO run_snapshot (root_run_id, snapshot_sha256) VALUES (?, ?)",
        )
        .bind(run)
        .bind(&snapshot.sha256)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}
