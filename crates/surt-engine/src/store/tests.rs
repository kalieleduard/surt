use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sqlx::AssertSqlSafe;
use tempfile::TempDir;

use super::{Store, StoreError};

const PLAN_SCHEMA: &[u8] =
    include_bytes!("../../../../docs/architecture/migrations/0001_initial.sql");

const PROJECT_NAME: &str = "my-app";

const PROJECT_ID_SUFFIX_DIGITS: usize = 6;

const SQLITE_SYNCHRONOUS_FULL: i64 = 2;

const SQLITE_BUSY_CODE: &str = "5";

const CONCURRENT_OPENERS: usize = 8;

const CONCURRENT_WRITERS: usize = 40;

const READ_THEN_INSERT_SUBJECT: [&str; 2] = [
    "SELECT count(*) FROM subject",
    "INSERT INTO subject (id, name, created_at) VALUES (lower(hex(randomblob(16))), lower(hex(randomblob(16))), 1792224000)",
];

const ROOT_RUN_FIXTURE: [&str; 4] = [
    "INSERT INTO subject (id, name, created_at) VALUES ('subject-1', '20261017-add-login', 1792224000)",
    "INSERT INTO snapshot (sha256, kind, name) VALUES ('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'workflow', 'synthetic')",
    "INSERT INTO snapshot (sha256, kind, name) VALUES ('cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'config', 'config')",
    "INSERT INTO workflow_run (id, subject_id, root_run_id, definition_sha256, ordinal, config_sha256, state, created_at) VALUES ('run-1', 'subject-1', 'run-1', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 1, 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'active', 1792224000)",
];

const SECOND_ACTIVE_ROOT_RUN: &str = "INSERT INTO workflow_run (id, subject_id, root_run_id, definition_sha256, ordinal, config_sha256, state, created_at) VALUES ('run-2', 'subject-1', 'run-2', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 2, 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc', 'active', 1792224100)";

const FINISH_FIRST_ROOT_RUN: &str =
    "UPDATE workflow_run SET state = 'succeeded', finished_at = 1792224050 WHERE id = 'run-1'";

const STEP_RUN: &str = "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, created_at, started_at) VALUES ('step-1', 'run-1', 'setup', 'operation', 'running', 1792224000, 1792224000)";

const RUNNING_FIRST_ATTEMPT: &str = "INSERT INTO execution (id, step_run_id, attempt, kind, operation_id, state, started_at) VALUES ('execution-1', 'step-1', 1, 'operation', 'git.prepare', 'running', 1792224000)";

const RUNNING_SECOND_ATTEMPT: &str = "INSERT INTO execution (id, step_run_id, attempt, kind, operation_id, state, started_at) VALUES ('execution-2', 'step-1', 2, 'operation', 'git.prepare', 'running', 1792224100)";

const FAIL_FIRST_ATTEMPT: &str = "UPDATE execution SET state = 'failed', failure_class = 'retryable', finished_at = 1792224050 WHERE id = 'execution-1'";

const PLAN_TABLES: [&str; 18] = [
    "approval",
    "artifact",
    "artifact_revision",
    "blob",
    "event",
    "execution",
    "execution_input",
    "project",
    "ref_move",
    "revision_file",
    "run_branch",
    "run_snapshot",
    "snapshot",
    "snapshot_file",
    "step_run",
    "subject",
    "workflow_run",
    "working_copy_file",
];

struct ProjectDir {
    root: TempDir,
}

impl ProjectDir {
    fn new() -> Self {
        Self {
            root: TempDir::new().expect("a temporary directory"),
        }
    }

    fn database(&self) -> PathBuf {
        self.root.path().join("surt.db")
    }

    fn lock(&self) -> PathBuf {
        self.root
            .path()
            .join("runtime")
            .join("locks")
            .join("db.lock")
    }

    async fn open(&self) -> Store {
        Store::open(&self.database(), &self.lock(), PROJECT_NAME)
            .await
            .expect("the store opens")
    }
}

async fn write(store: &Store, statements: &[&'static str]) -> Result<(), sqlx::Error> {
    let mut transaction = store.begin_write().await?;
    for statement in statements {
        sqlx::query(*statement).execute(&mut *transaction).await?;
    }
    transaction.commit().await
}

fn is_unique_violation(result: &Result<(), sqlx::Error>) -> bool {
    matches!(result, Err(sqlx::Error::Database(error)) if error.is_unique_violation())
}

async fn count(store: &Store, table: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(&store.pool)
        .await
        .expect("the table is readable")
}

async fn strict_tables(store: &Store) -> Vec<(String, bool)> {
    sqlx::query_as::<_, (String, bool)>(
        "SELECT name, strict FROM pragma_table_list WHERE schema = 'main' AND type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(&store.pool)
    .await
    .expect("the table list is readable")
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_open_creates_every_plan_table_as_strict() {
    let dir = ProjectDir::new();

    let store = dir.open().await;

    let expected: Vec<(String, bool)> = PLAN_TABLES
        .iter()
        .map(|name| ((*name).to_owned(), true))
        .collect();
    assert_eq!(strict_tables(&store).await, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn every_connection_uses_wal_full_sync_and_foreign_keys() {
    let dir = ProjectDir::new();
    let store = dir.open().await;

    let (journal, synchronous, foreign_keys) = sqlx::query_as::<_, (String, i64, bool)>(
        "SELECT (SELECT journal_mode FROM pragma_journal_mode), (SELECT synchronous FROM pragma_synchronous), (SELECT foreign_keys FROM pragma_foreign_keys)",
    )
    .fetch_one(&store.pool)
    .await
    .expect("the pragmas are readable");

    assert_eq!(journal, "wal");
    assert_eq!(synchronous, SQLITE_SYNCHRONOUS_FULL);
    assert!(foreign_keys);
}

#[test]
fn the_embedded_migration_is_the_plan_schema_byte_for_byte() {
    let embedded: Vec<&[u8]> = super::MIGRATOR
        .iter()
        .map(|migration| migration.sql.as_ref().as_bytes())
        .collect();

    assert_eq!(embedded, [PLAN_SCHEMA]);
    assert!(!PLAN_SCHEMA.contains(&b'\r'));
}

#[tokio::test(flavor = "multi_thread")]
async fn reopening_keeps_the_data_and_applies_no_migration_twice() {
    let dir = ProjectDir::new();
    let first = dir.open().await;
    write(
        &first,
        &[
            "INSERT INTO subject (id, name, created_at) VALUES ('subject-1', '20261017-add-login', 1792224000)",
        ],
    )
    .await
    .expect("the subject is written");
    first.pool.close().await;

    let reopened = dir.open().await;

    assert_eq!(count(&reopened, "subject").await, 1);
    assert_eq!(count(&reopened, "_sqlx_migrations").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn creating_the_database_records_a_project_id_of_the_name_and_six_hex_digits() {
    let dir = ProjectDir::new();

    let store = dir.open().await;

    let id = store
        .project_id()
        .await
        .expect("the project id is readable");
    let suffix = id
        .strip_prefix(&format!("{PROJECT_NAME}-"))
        .expect("the id starts with the project name");
    assert_eq!(suffix.len(), PROJECT_ID_SUFFIX_DIGITS, "{id}");
    assert!(
        suffix
            .chars()
            .all(|digit| digit.is_ascii_digit() || ('a'..='f').contains(&digit)),
        "{id}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn reopening_keeps_the_project_id() {
    let dir = ProjectDir::new();
    let first = dir.open().await;
    let created = first
        .project_id()
        .await
        .expect("the project id is readable");
    first.pool.close().await;

    let reopened = dir.open().await;

    assert_eq!(reopened.project_id().await.ok(), Some(created));
    assert_eq!(count(&reopened, "project").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subject_cannot_have_two_active_root_runs() {
    let dir = ProjectDir::new();
    let store = dir.open().await;
    write(&store, &ROOT_RUN_FIXTURE)
        .await
        .expect("the fixture is written");

    let second = write(&store, &[SECOND_ACTIVE_ROOT_RUN]).await;

    assert!(is_unique_violation(&second), "{second:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subject_gets_a_new_active_root_run_once_the_previous_one_finished() {
    let dir = ProjectDir::new();
    let store = dir.open().await;
    write(&store, &ROOT_RUN_FIXTURE)
        .await
        .expect("the fixture is written");

    let second = write(&store, &[FINISH_FIRST_ROOT_RUN, SECOND_ACTIVE_ROOT_RUN]).await;

    assert!(second.is_ok(), "{second:?}");
    assert_eq!(count(&store, "workflow_run").await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_run_cannot_have_two_live_executions() {
    let dir = ProjectDir::new();
    let store = dir.open().await;
    write(&store, &ROOT_RUN_FIXTURE)
        .await
        .expect("the fixture is written");
    write(&store, &[STEP_RUN, RUNNING_FIRST_ATTEMPT])
        .await
        .expect("the first attempt is written");

    let second = write(&store, &[RUNNING_SECOND_ATTEMPT]).await;

    assert!(is_unique_violation(&second), "{second:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_run_gets_a_new_live_execution_once_the_previous_one_ended() {
    let dir = ProjectDir::new();
    let store = dir.open().await;
    write(&store, &ROOT_RUN_FIXTURE)
        .await
        .expect("the fixture is written");
    write(&store, &[STEP_RUN, RUNNING_FIRST_ATTEMPT])
        .await
        .expect("the first attempt is written");

    let second = write(&store, &[FAIL_FIRST_ATTEMPT, RUNNING_SECOND_ATTEMPT]).await;

    assert!(second.is_ok(), "{second:?}");
    assert_eq!(count(&store, "execution").await, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_writer_gives_up_busy_after_its_retries_while_another_store_holds_the_write_lock() {
    let dir = ProjectDir::new();
    let holder = dir.open().await;
    let impatient =
        Store::open_with_busy_timeout(&dir.database(), &dir.lock(), PROJECT_NAME, Duration::ZERO)
            .await
            .expect("the store opens");
    let held = holder.begin_write().await.expect("the write lock is taken");

    let refused = impatient.begin_write().await.err();

    let code = match &refused {
        Some(sqlx::Error::Database(error)) => error.code().map(|code| code.into_owned()),
        _ => None,
    };
    assert_eq!(code.as_deref(), Some(SQLITE_BUSY_CODE), "{refused:?}");
    held.rollback().await.expect("the write lock is released");
}

#[tokio::test(flavor = "multi_thread")]
async fn read_then_write_transactions_from_two_stores_all_commit() {
    let dir = ProjectDir::new();
    let stores = [Arc::new(dir.open().await), Arc::new(dir.open().await)];

    let writers: Vec<_> = (0..CONCURRENT_WRITERS)
        .map(|writer| {
            let store = Arc::clone(&stores[writer % stores.len()]);
            tokio::spawn(async move { write(&store, &READ_THEN_INSERT_SUBJECT).await })
        })
        .collect();
    let mut failures = Vec::new();
    for writer in writers {
        if let Err(error) = writer.await.expect("the writer finishes") {
            failures.push(error);
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(
        count(&stores[0], "subject").await,
        i64::try_from(CONCURRENT_WRITERS).expect("a small count")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_openers_of_a_new_database_all_succeed() {
    let dir = Arc::new(ProjectDir::new());

    let openers: Vec<_> = (0..CONCURRENT_OPENERS)
        .map(|_| {
            let dir = Arc::clone(&dir);
            tokio::spawn(
                async move { Store::open(&dir.database(), &dir.lock(), PROJECT_NAME).await },
            )
        })
        .collect();
    let mut failures = Vec::new();
    for opener in openers {
        if let Err(error) = opener.await.expect("the opener finishes") {
            failures.push(error);
        }
    }

    assert!(failures.is_empty(), "{failures:?}");
    let store = dir.open().await;
    assert_eq!(count(&store, "_sqlx_migrations").await, 1);
    assert_eq!(count(&store, "project").await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_migrated_by_a_newer_binary_is_refused() {
    let dir = ProjectDir::new();
    let store = dir.open().await;
    write(
        &store,
        &[
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (7, 'from a newer surt', 1, x'00', 0)",
        ],
    )
    .await
    .expect("the newer migration is recorded");
    store.pool.close().await;

    let refusal = Store::open(&dir.database(), &dir.lock(), PROJECT_NAME)
        .await
        .err();
    let reading = Store::open_read_only(&dir.database()).await.err();

    for refused in [refusal, reading] {
        assert!(
            matches!(
                refused,
                Some(StoreError::SchemaTooNew {
                    db: 7,
                    supported: 1
                })
            ),
            "{refused:?}"
        );
    }
}
