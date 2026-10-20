use std::collections::BTreeMap;
use std::path::Path;

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{AssertSqlSafe, SqlitePool};

use super::{PLAN_TABLES, ProjectDir, write};
use crate::store::{BUSY_TIMEOUT, MIGRATOR, connect_options, migrate};

mod fixture;

const INITIAL_SCHEMA: i64 = 1;

const MIGRATED_SCHEMA: i64 = 2;

const INPUT_STEP_RUN: &str = "INSERT INTO step_run (id, workflow_run_id, step_key, kind, state, created_at) VALUES ('step-answer', 'run-1', 'answer-spec', 'input', 'pending', 1792224300)";

const ANSWER_REVISION: &str = "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, human_action, actor, created_at) VALUES ('revision-answer', 'artifact-spec', 3, 'abababababababababababababababababababababababababababababababab', 'human', 'answer', 'dev@example.com', 1792224300)";

const WIDENED_VALUE_LISTS: [(&str, &str); 2] = [
    ("'loop', 'per_item')", "'loop', 'per_item', 'input')"),
    (
        "'edit', 'rejection_reason')",
        "'edit', 'rejection_reason', 'answer')",
    ),
];

const RENAMED_TABLES: [(&str, &str); 2] = [
    ("CREATE TABLE step_run (", "CREATE TABLE \"step_run\" ("),
    (
        "CREATE TABLE artifact_revision (",
        "CREATE TABLE \"artifact_revision\" (",
    ),
];

type Rows = BTreeMap<&'static str, Vec<String>>;

async fn single_connection(database: &Path) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(connect_options(database, BUSY_TIMEOUT))
        .await
        .expect("the database opens")
}

async fn initial_database(database: &Path) -> SqlitePool {
    let pool = single_connection(database).await;
    MIGRATOR
        .run_to(INITIAL_SCHEMA, &pool)
        .await
        .expect("0001 applies");
    pool
}

async fn every_row(pool: &SqlitePool) -> Rows {
    let mut rows = Rows::new();
    for table in PLAN_TABLES {
        let columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
                .bind(table)
                .fetch_all(pool)
                .await
                .expect("the columns are readable");
        let row = columns
            .iter()
            .map(|column| format!("quote({column})"))
            .collect::<Vec<_>>()
            .join(" || '|' || ");
        let values = sqlx::query_scalar(AssertSqlSafe(format!(
            "SELECT {row} AS row FROM {table} ORDER BY row"
        )))
        .fetch_all(pool)
        .await
        .expect("the rows are readable");
        rows.insert(table, values);
    }
    rows
}

async fn schema(pool: &SqlitePool) -> Vec<(String, String, Option<String>)> {
    sqlx::query_as(
        "SELECT type, name, sql FROM sqlite_schema WHERE tbl_name <> '_sqlx_migrations' ORDER BY type, name",
    )
    .fetch_all(pool)
    .await
    .expect("the schema is readable")
}

async fn applied_versions(pool: &SqlitePool) -> Vec<i64> {
    sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await
        .expect("the migrations are readable")
}

fn rewritten(sql: &str, replacements: &[(&str, &str)]) -> String {
    replacements
        .iter()
        .fold(sql.to_owned(), |sql, (from, to)| sql.replace(from, to))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_database_created_by_0001_keeps_every_row_and_foreign_key_through_0002() {
    let dir = ProjectDir::new();
    let initial = initial_database(&dir.database()).await;
    fixture::fill(&initial).await;
    let before = every_row(&initial).await;
    initial.close().await;

    let store = dir.open().await;

    assert!(before.values().all(|rows| !rows.is_empty()), "{before:?}");
    assert_eq!(every_row(&store.pool).await, before);
    assert_eq!(
        applied_versions(&store.pool).await,
        [INITIAL_SCHEMA, MIGRATED_SCHEMA]
    );
    let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(&store.pool)
        .await
        .expect("the integrity check runs");
    assert_eq!(integrity, ["ok"]);
    let violations = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&store.pool)
        .await
        .expect("the foreign key check runs");
    assert!(violations.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_step_and_an_answer_revision_insert_after_0002() {
    let dir = ProjectDir::new();
    let initial = initial_database(&dir.database()).await;
    fixture::fill(&initial).await;
    initial.close().await;
    let store = dir.open().await;

    let inserted = write(&store, &[INPUT_STEP_RUN, ANSWER_REVISION]).await;

    assert!(inserted.is_ok(), "{inserted:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn migration_0002_widens_only_the_step_kinds_and_the_human_actions() {
    let dir = ProjectDir::new();
    let pool = initial_database(&dir.database()).await;
    let replacements = [WIDENED_VALUE_LISTS, RENAMED_TABLES].concat();
    let expected: Vec<_> = schema(&pool)
        .await
        .into_iter()
        .map(|(kind, name, sql)| (kind, name, sql.map(|sql| rewritten(&sql, &replacements))))
        .collect();

    migrate(&pool).await.expect("0002 applies");

    assert_eq!(schema(&pool).await, expected);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_connection_that_ran_0002_goes_back_to_the_pool_with_foreign_keys_on() {
    let dir = ProjectDir::new();
    let pool = initial_database(&dir.database()).await;

    migrate(&pool).await.expect("0002 applies");

    let foreign_keys: bool = sqlx::query_scalar("SELECT foreign_keys FROM pragma_foreign_keys")
        .fetch_one(&pool)
        .await
        .expect("the pragma is readable");
    assert!(foreign_keys);
}
