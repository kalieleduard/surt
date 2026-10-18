use sqlx::FromRow;

use super::{RootRunRow, Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct SubjectRow {
    pub(crate) name: String,
    pub(crate) active_run: Option<String>,
    pub(crate) pending_cleanup: Option<String>,
}

impl Store {
    pub(crate) async fn root_runs(&self, subject: &str) -> Result<Vec<RootRunRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT w.id, s.name AS subject, w.ordinal, w.state, w.definition_sha256 AS definition, \
             w.cleaned_at IS NOT NULL AS cleaned, \
             w.driver_pid, w.driver_start_time, w.driver_boot_id, w.driver_since, w.finished_at, \
             (SELECT count(*) FROM workflow_run c WHERE c.subject_id = w.subject_id \
              AND c.parent_step_run_id IS NULL) AS runs \
             FROM subject s JOIN workflow_run w ON w.subject_id = s.id AND w.parent_step_run_id IS NULL \
             WHERE s.name = ? ORDER BY w.ordinal",
        )
        .bind(subject)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn subjects(&self) -> Result<Vec<SubjectRow>, sqlx::Error> {
        sqlx::query_as(
            "SELECT s.name, w.id AS active_run, \
             (SELECT p.id FROM workflow_run p WHERE p.subject_id = s.id AND p.parent_step_run_id IS NULL \
              AND p.state <> 'active' AND p.cleaned_at IS NULL ORDER BY p.ordinal DESC LIMIT 1) AS pending_cleanup \
             FROM subject s LEFT JOIN workflow_run w \
             ON w.subject_id = s.id AND w.parent_step_run_id IS NULL AND w.state = 'active' \
             ORDER BY s.name",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub(crate) async fn names_of_subjects_with_active_runs(
        &self,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT subject.name FROM subject JOIN workflow_run ON workflow_run.subject_id = subject.id \
             WHERE workflow_run.parent_step_run_id IS NULL AND workflow_run.state = 'active' \
             ORDER BY subject.name DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub(crate) async fn subject_names_starting_with(
        &self,
        stem: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT name FROM subject WHERE substr(name, 1, length(?1)) = ?1")
            .bind(stem)
            .fetch_all(&self.pool)
            .await
    }

    pub(crate) async fn subject_of_run(&self, run: &str) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT s.name FROM subject s JOIN workflow_run w ON w.subject_id = s.id WHERE w.id = ?",
        )
        .bind(run)
        .fetch_one(&self.pool)
        .await
    }
}
