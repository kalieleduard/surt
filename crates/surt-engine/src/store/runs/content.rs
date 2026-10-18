use sqlx::SqliteConnection;

use super::{NewInitialArtifact, NewRunRows};
use crate::store::revisions::{Origin, artifact_id, insert_blob, publish};
use crate::workflow::Snapshot;

const INITIAL: &str = "initial";

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
    let artifact_id = artifact_id(&mut *connection, run, &artifact.name).await?;
    let origin = Origin::Human {
        action: INITIAL,
        actor: &rows.actor,
    };
    publish(connection, &artifact_id, &artifact.files, origin, rows.now).await?;
    Ok(())
}
