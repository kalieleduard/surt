use sqlx::FromRow;

use super::{ExecutionScopeRow, Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct LogSourceRow {
    #[sqlx(flatten)]
    pub(crate) scope: ExecutionScopeRow,
    pub(crate) run: String,
    pub(crate) kind: String,
}

impl Store {
    pub(crate) async fn log_sources(
        &self,
        root: &str,
        item_run: Option<&str>,
    ) -> Result<Vec<LogSourceRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT x.id, sub.name AS subject, root.ordinal, w.item_key AS item, \
             s.step_key AS step, s.iteration, x.attempt, x.state, w.id AS run, x.kind \
             FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             JOIN workflow_run root ON root.id = w.root_run_id \
             JOIN subject sub ON sub.id = root.subject_id \
             WHERE w.root_run_id = ?1 AND (?2 IS NULL OR w.id = ?2) \
             ORDER BY w.item_key IS NOT NULL, w.item_key, x.rowid",
        )
        .bind(root)
        .bind(item_run)
        .fetch_all(&self.pool)
        .await?)
    }
}
