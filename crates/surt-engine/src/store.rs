use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::platform::{self, ExclusiveLock};

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
}

pub(crate) struct Store {
    pool: SqlitePool,
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
        let store = Store { pool };
        store.record_project(project_name).await?;
        Ok(store)
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

fn unix_now() -> i64 {
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
