use super::{Store, StoreError};

impl Store {
    pub(crate) async fn last_operation_done(
        &self,
        run: &str,
        operation: &str,
    ) -> Result<Option<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT v.data FROM event v JOIN step_run s ON s.id = v.step_run_id \
             WHERE s.workflow_run_id = ?1 AND v.kind = 'step.done' AND EXISTS \
             (SELECT 1 FROM execution e WHERE e.step_run_id = s.id AND e.operation_id = ?2) \
             ORDER BY v.seq DESC LIMIT 1",
        )
        .bind(run)
        .bind(operation)
        .fetch_optional(&self.pool)
        .await?)
    }
}
