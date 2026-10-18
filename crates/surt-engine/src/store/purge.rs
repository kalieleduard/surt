use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct SubjectRun {
    pub(crate) id: String,
    pub(crate) ordinal: i64,
    pub(crate) state: String,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct SubjectBranch {
    pub(crate) ref_name: String,
    pub(crate) worktree_path: Option<String>,
    pub(crate) ordinal: i64,
    pub(crate) root_state: String,
    pub(crate) item: bool,
}

#[derive(Clone, Copy, Debug, Default, FromRow)]
pub(crate) struct HistoryCounts {
    pub(crate) revisions: i64,
    pub(crate) executions: i64,
    pub(crate) events: i64,
}

const ROOT_RUNS: &str = "SELECT w.id, w.ordinal, w.state FROM workflow_run w \
     JOIN subject s ON s.id = w.subject_id \
     WHERE s.name = ? AND w.parent_step_run_id IS NULL ORDER BY w.ordinal";

const UNREFERENCED_SNAPSHOTS: &str = "DELETE FROM snapshot WHERE \
     sha256 NOT IN (SELECT snapshot_sha256 FROM run_snapshot) \
     AND sha256 NOT IN (SELECT definition_sha256 FROM workflow_run) \
     AND sha256 NOT IN (SELECT config_sha256 FROM workflow_run WHERE config_sha256 IS NOT NULL) \
     AND sha256 NOT IN (SELECT agent_snapshot_sha256 FROM execution \
                        WHERE agent_snapshot_sha256 IS NOT NULL)";

const UNREFERENCED_BLOBS: &str = "DELETE FROM blob WHERE \
     sha256 NOT IN (SELECT blob_sha256 FROM revision_file) \
     AND sha256 NOT IN (SELECT blob_sha256 FROM snapshot_file)";

impl Store {
    pub(crate) async fn subject_runs(&self, subject: &str) -> Result<Vec<SubjectRun>, StoreError> {
        Ok(sqlx::query_as(ROOT_RUNS)
            .bind(subject)
            .fetch_all(&self.pool)
            .await?)
    }

    pub(crate) async fn subject_branches(
        &self,
        subject: &str,
    ) -> Result<Vec<SubjectBranch>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT b.ref_name, b.worktree_path, r.ordinal, r.state AS root_state, \
             w.parent_step_run_id IS NOT NULL AS item \
             FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
             JOIN workflow_run r ON r.id = w.root_run_id JOIN subject s ON s.id = r.subject_id \
             WHERE s.name = ? ORDER BY r.ordinal, w.parent_step_run_id IS NOT NULL, b.ref_name",
        )
        .bind(subject)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn subject_executions(
        &self,
        subject: &str,
    ) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT e.id FROM execution e JOIN step_run t ON t.id = e.step_run_id \
             JOIN workflow_run w ON w.id = t.workflow_run_id \
             JOIN subject s ON s.id = w.subject_id WHERE s.name = ? ORDER BY e.id",
        )
        .bind(subject)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn subject_run_ids(&self, subject: &str) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT w.id FROM workflow_run w JOIN subject s ON s.id = w.subject_id \
             WHERE s.name = ? ORDER BY w.id",
        )
        .bind(subject)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn history_counts(&self, subject: &str) -> Result<HistoryCounts, StoreError> {
        Ok(sqlx::query_as(
            "SELECT \
             (SELECT count(*) FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
              JOIN workflow_run w ON w.id = a.workflow_run_id \
              WHERE w.subject_id = s.id) AS revisions, \
             (SELECT count(*) FROM execution e JOIN step_run t ON t.id = e.step_run_id \
              JOIN workflow_run w ON w.id = t.workflow_run_id \
              WHERE w.subject_id = s.id) AS executions, \
             (SELECT count(*) FROM event v JOIN workflow_run w ON w.id = v.root_run_id \
              WHERE w.subject_id = s.id) AS events \
             FROM subject s WHERE s.name = ?",
        )
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_default())
    }

    pub(crate) async fn purge_subject(
        &self,
        subject: &str,
        cleaned: &[SubjectRun],
    ) -> Result<bool, StoreError> {
        let mut transaction = self.begin_write().await?;
        let current: Vec<SubjectRun> = sqlx::query_as(ROOT_RUNS)
            .bind(subject)
            .fetch_all(&mut *transaction)
            .await?;
        if current != cleaned {
            transaction.rollback().await?;
            return Ok(false);
        }
        sqlx::query("DELETE FROM subject WHERE name = ?")
            .bind(subject)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(UNREFERENCED_SNAPSHOTS)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(UNREFERENCED_BLOBS)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(true)
    }
}
