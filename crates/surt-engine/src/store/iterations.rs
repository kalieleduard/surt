use super::columns::number;
use super::{Store, StoreError};

impl Store {
    pub(crate) async fn revision_in_iteration(
        &self,
        run: &str,
        artifact: &str,
        iteration: u32,
    ) -> Result<Option<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             LEFT JOIN execution e ON e.id = r.execution_id \
             LEFT JOIN step_run s ON s.id = e.step_run_id \
             LEFT JOIN approval p ON p.reason_revision_id = r.id \
             LEFT JOIN step_run g ON g.id = p.gate_step_run_id \
             WHERE a.workflow_run_id = ?1 AND a.name = ?2 \
             AND coalesce(s.iteration, g.iteration) = ?3 \
             ORDER BY r.number DESC LIMIT 1",
        )
        .bind(run)
        .bind(artifact)
        .bind(iteration)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn revision_iterations(
        &self,
        run: &str,
        artifact: &str,
    ) -> Result<Vec<u32>, StoreError> {
        let iterations: Vec<i64> = sqlx::query_scalar(
            "SELECT DISTINCT coalesce(s.iteration, g.iteration) AS produced \
             FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
             LEFT JOIN execution e ON e.id = r.execution_id \
             LEFT JOIN step_run s ON s.id = e.step_run_id \
             LEFT JOIN approval p ON p.reason_revision_id = r.id \
             LEFT JOIN step_run g ON g.id = p.gate_step_run_id \
             WHERE a.workflow_run_id = ?1 AND a.name = ?2 AND produced IS NOT NULL \
             ORDER BY produced",
        )
        .bind(run)
        .bind(artifact)
        .fetch_all(&self.pool)
        .await?;
        Ok(iterations.into_iter().map(number).collect())
    }
}
