use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Sqlite, SqliteConnection, SqlitePool, Transaction};

use crate::platform::{self, ExclusiveLock};

mod acceptance;
mod approvals;
mod blocked;
mod branches;
mod cancelling;
mod checkpoints;
mod cleanup;
mod control;
mod details;
mod drivers;
mod events;
mod executions;
mod frozen;
mod graph;
mod items;
mod iterations;
mod reading;
mod ref_moves;
mod restoring;
mod revisions;
mod runs;
mod runtime_info;
mod steps;
mod subjects;
mod working_copy;

pub(crate) use acceptance::{Acceptance, Integrated, PublishedOutput};
pub(crate) use approvals::{Decision, DecisionOutcome, GateRow, NewDecision, WaitingGate};
pub(crate) use blocked::{BlockedRow, FailureRow};
pub(crate) use branches::{PreviousExecution, RunBranch};
pub use checkpoints::Checkpoint;
#[cfg(feature = "test-support")]
pub(crate) use checkpoints::Faults;
pub(crate) use cleanup::{ChildBranch, CleanupRecord};
pub(crate) use drivers::StaleExecution;
pub(crate) use executions::{
    ExecutionEnd, ExecutionKind, ExecutionStart, InputRequest, StartedExecution,
};
pub(crate) use frozen::FrozenFile;
pub(crate) use graph::{RunRows, StepRow};
pub(crate) use items::{Expansion, ListRevision, NewChild};
pub(crate) use reading::{ArtifactSummaryRow, GateViewRow, ItemRunRow, RootRunRow};
pub(crate) use ref_moves::{PendingRefMove, RefMove};
pub(crate) use restoring::RestoreTarget;
pub(crate) use revisions::{Producer, StoredRevision};
pub(crate) use runs::{NewInitialArtifact, NewRunRows, RunCreation};
pub(crate) use subjects::SubjectRow;
pub(crate) use working_copy::{BaselineFile, EditRecord, PendingFile, WorkingCopyArtifact};

static MIGRATOR: Migrator = sqlx::migrate!();

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BEGIN_ATTEMPTS: u32 = 5;
const BUSY_RETRY_DELAY: Duration = Duration::from_millis(50);
const SQLITE_BUSY: i32 = 5;
const PRIMARY_RESULT_CODE_MASK: i32 = 0xff;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
    #[error("database schema {db} is newer than the supported schema {supported}")]
    SchemaTooNew { db: i64, supported: i64 },
    #[error("cannot take the database lock: {0}")]
    Lock(#[source] io::Error),
    #[error(transparent)]
    Migration(#[from] MigrateError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[cfg(feature = "test-support")]
    #[error("crashed after {0:?}")]
    Crashed(Checkpoint),
}

pub(crate) struct Store {
    pool: SqlitePool,
    #[cfg(feature = "test-support")]
    faults: Faults,
}

impl Store {
    pub(crate) async fn open(
        database: &Path,
        lock: &Path,
        project_name: &str,
    ) -> Result<Store, StoreError> {
        Self::open_with_busy_timeout(database, lock, project_name, BUSY_TIMEOUT).await
    }

    async fn open_with_busy_timeout(
        database: &Path,
        lock: &Path,
        project_name: &str,
        busy_timeout: Duration,
    ) -> Result<Store, StoreError> {
        tracing::debug!(database = %database.display(), "opening the database");
        let _migration_lock = take_lock(lock.to_path_buf()).await?;
        let pool = SqlitePool::connect_with(connect_options(database, busy_timeout)).await?;
        migrate(&pool).await?;
        let store = Store {
            pool,
            #[cfg(feature = "test-support")]
            faults: Faults::default(),
        };
        store.record_project(project_name).await?;
        Ok(store)
    }

    pub(crate) async fn open_read_only(database: &Path) -> Result<Store, StoreError> {
        tracing::debug!(database = %database.display(), "opening the database read-only");
        let options = SqliteConnectOptions::new()
            .filename(database)
            .read_only(true)
            .busy_timeout(BUSY_TIMEOUT);
        let pool = SqlitePool::connect_with(options).await?;
        let db = applied_version(&pool).await?;
        let supported = supported_version();
        if db > supported {
            pool.close().await;
            return Err(StoreError::SchemaTooNew { db, supported });
        }
        Ok(Store {
            pool,
            #[cfg(feature = "test-support")]
            faults: Faults::default(),
        })
    }

    pub(crate) async fn close(self) {
        self.pool.close().await;
    }

    pub(crate) async fn project_id(&self) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("SELECT project_id FROM project")
            .fetch_one(&self.pool)
            .await
    }

    async fn record_project(&self, project_name: &str) -> Result<(), sqlx::Error> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "INSERT INTO project (project_id, created_at) SELECT ?1 || '-' || lower(hex(randomblob(3))), ?2 WHERE NOT EXISTS (SELECT 1 FROM project)",
        )
        .bind(project_name)
        .bind(unix_now())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await
    }

    async fn begin_write(&self) -> Result<Transaction<'static, Sqlite>, sqlx::Error> {
        let mut attempt = 1;
        loop {
            match self.pool.begin_with("BEGIN IMMEDIATE").await {
                Err(error) if is_busy(&error) && attempt < MAX_BEGIN_ATTEMPTS => {
                    attempt += 1;
                    tokio::time::sleep(BUSY_RETRY_DELAY).await;
                }
                result => return result,
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum About<'a> {
    Run(&'a str),
    Step(&'a str),
    Execution(&'a str),
}

pub(super) async fn record_event(
    connection: &mut SqliteConnection,
    about: About<'_>,
    kind: &str,
    at: i64,
) -> Result<(), sqlx::Error> {
    record_event_data(connection, about, kind, at, &serde_json::json!({})).await
}

pub(super) async fn record_event_data(
    connection: &mut SqliteConnection,
    about: About<'_>,
    kind: &str,
    at: i64,
    data: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let (sql, id) = match about {
        About::Run(id) => (
            "INSERT INTO event (at, root_run_id, workflow_run_id, kind, data) \
             SELECT ?1, w.root_run_id, w.id, ?2, ?4 FROM workflow_run w WHERE w.id = ?3",
            id,
        ),
        About::Step(id) => (
            "INSERT INTO event (at, root_run_id, workflow_run_id, step_run_id, kind, data) \
             SELECT ?1, w.root_run_id, w.id, s.id, ?2, ?4 FROM step_run s \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE s.id = ?3",
            id,
        ),
        About::Execution(id) => (
            "INSERT INTO event (at, root_run_id, workflow_run_id, step_run_id, execution_id, kind, data) \
             SELECT ?1, w.root_run_id, w.id, s.id, e.id, ?2, ?4 FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE e.id = ?3",
            id,
        ),
    };
    sqlx::query(sql)
        .bind(at)
        .bind(kind)
        .bind(id)
        .bind(data.to_string())
        .execute(connection)
        .await?;
    Ok(())
}

pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

fn is_busy(error: &sqlx::Error) -> bool {
    let sqlx::Error::Database(database) = error else {
        return false;
    };
    database
        .code()
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & PRIMARY_RESULT_CODE_MASK == SQLITE_BUSY)
}

fn connect_options(database: &Path, busy_timeout: Duration) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(database)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Full)
        .foreign_keys(true)
        .busy_timeout(busy_timeout)
}

async fn take_lock(path: PathBuf) -> Result<ExclusiveLock, StoreError> {
    tokio::task::spawn_blocking(move || platform::lock_exclusive(&path))
        .await
        .map_err(|join| StoreError::Lock(io::Error::other(join)))?
        .map_err(StoreError::Lock)
}

async fn migrate(pool: &SqlitePool) -> Result<(), StoreError> {
    match MIGRATOR.run(pool).await {
        Err(MigrateError::VersionMissing(_)) => Err(StoreError::SchemaTooNew {
            db: applied_version(pool).await?,
            supported: supported_version(),
        }),
        result => Ok(result?),
    }
}

async fn applied_version(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
}

fn supported_version() -> i64 {
    MIGRATOR
        .iter()
        .map(|migration| migration.version)
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
