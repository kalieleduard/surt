use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::platform::{self, ExclusiveLock};

mod runs;

pub(crate) use runs::{NewInitialArtifact, NewRunRows, RunCreation};

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
    pub(crate) async fn open(database: &Path, lock: &Path) -> Result<Store, StoreError> {
        Self::open_with_busy_timeout(database, lock, BUSY_TIMEOUT).await
    }

    async fn open_with_busy_timeout(
        database: &Path,
        lock: &Path,
        busy_timeout: Duration,
    ) -> Result<Store, StoreError> {
        let _migration_lock = take_lock(lock.to_path_buf()).await?;
        let pool = SqlitePool::connect_with(connect_options(database, busy_timeout)).await?;
        migrate(&pool).await?;
        Ok(Store { pool })
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
