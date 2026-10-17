use sqlx::SqliteConnection;

use super::{NewInitialArtifact, NewRunRows, new_id};
use crate::artifacts::{identity, sha256_hex};
use crate::workflow::Snapshot;

const FIRST_REVISION: i64 = 1;

async fn insert_blob(
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

pub(super) async fn insert_snapshot(
    connection: &mut SqliteConnection,
    snapshot: &Snapshot,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR IGNORE INTO snapshot (sha256, kind, name) VALUES (?, ?, ?)")
        .bind(&snapshot.sha256)
        .bind(snapshot.kind.as_str())
        .bind(&snapshot.name)
        .execute(&mut *connection)
        .await?;
    for file in &snapshot.files {
        insert_blob(&mut *connection, &file.sha256, &file.bytes).await?;
        sqlx::query("INSERT OR IGNORE INTO snapshot_file (snapshot_sha256, rel_path, blob_sha256) VALUES (?, ?, ?)")
            .bind(&snapshot.sha256)
            .bind(&file.rel_path)
            .bind(&file.sha256)
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}

pub(super) async fn insert_initial_artifact(
    connection: &mut SqliteConnection,
    run: &str,
    artifact: &NewInitialArtifact,
    rows: &NewRunRows,
) -> Result<(), sqlx::Error> {
    let artifact_id = new_id();
    sqlx::query("INSERT INTO artifact (id, workflow_run_id, name) VALUES (?, ?, ?)")
        .bind(&artifact_id)
        .bind(run)
        .bind(&artifact.name)
        .execute(&mut *connection)
        .await?;
    let files: Vec<(&str, String, &[u8])> = artifact
        .files
        .iter()
        .map(|(rel_path, bytes)| (rel_path.as_str(), sha256_hex(bytes), bytes.as_slice()))
        .collect();
    let revision = new_id();
    sqlx::query(
        "INSERT INTO artifact_revision (id, artifact_id, number, identity_sha256, origin, human_action, actor, created_at) \
         VALUES (?, ?, ?, ?, 'human', 'initial', ?, ?)",
    )
    .bind(&revision)
    .bind(&artifact_id)
    .bind(FIRST_REVISION)
    .bind(identity(files.iter().map(|(rel_path, sha256, _)| (*rel_path, sha256.as_str()))))
    .bind(&rows.actor)
    .bind(rows.now)
    .execute(&mut *connection)
    .await?;
    for (rel_path, sha256, bytes) in &files {
        insert_blob(&mut *connection, sha256, bytes).await?;
        sqlx::query(
            "INSERT INTO revision_file (revision_id, rel_path, blob_sha256) VALUES (?, ?, ?)",
        )
        .bind(&revision)
        .bind(rel_path)
        .bind(sha256)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}
