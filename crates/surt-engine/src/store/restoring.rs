use sqlx::FromRow;

use super::revisions::number;
use super::{BaselineFile, Store, StoreError};

pub(crate) struct RestoreTarget {
    pub(crate) revision: u32,
    pub(crate) files: Vec<BaselineFile>,
}

#[derive(FromRow)]
struct TargetRow {
    rel_path: String,
    sha256: String,
    revision: i64,
}

impl Store {
    pub(crate) async fn restore_target(
        &self,
        root: &str,
        artifact: &str,
    ) -> Result<Option<RestoreTarget>, StoreError> {
        let rows: Vec<TargetRow> = sqlx::query_as(
            "SELECT w.rel_path, coalesce(w.pending_sha256, w.sha256) AS sha256, r.number AS revision \
             FROM working_copy_file w JOIN artifact a ON a.id = w.artifact_id \
             JOIN artifact_revision r ON r.id = coalesce(w.pending_revision_id, w.revision_id) \
             WHERE a.workflow_run_id = ?1 AND a.name = ?2 AND (w.pending_revision_id IS NOT NULL \
             OR NOT EXISTS (SELECT 1 FROM working_copy_file p WHERE p.artifact_id = w.artifact_id \
             AND p.pending_revision_id IS NOT NULL)) ORDER BY w.rel_path",
        )
        .bind(root)
        .bind(artifact)
        .fetch_all(&self.pool)
        .await?;
        let Some(revision) = rows.iter().map(|row| row.revision).max() else {
            return Ok(None);
        };
        let files = rows
            .into_iter()
            .map(|row| BaselineFile {
                rel_path: row.rel_path,
                sha256: row.sha256,
            })
            .collect();
        Ok(Some(RestoreTarget {
            revision: number(revision),
            files,
        }))
    }

    pub(crate) async fn restorable(&self, root: &str) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT DISTINCT a.name FROM working_copy_file w JOIN artifact a ON a.id = w.artifact_id \
             WHERE a.workflow_run_id = ? ORDER BY a.name",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }
}
