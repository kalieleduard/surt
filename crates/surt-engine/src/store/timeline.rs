use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct EventRow {
    pub(crate) at: i64,
    pub(crate) kind: String,
    pub(crate) item: Option<String>,
    pub(crate) step: Option<String>,
    pub(crate) iteration: Option<i64>,
    pub(crate) attempt: Option<i64>,
    pub(crate) execution: Option<String>,
    pub(crate) data: String,
}

impl Store {
    pub(crate) async fn timeline(
        &self,
        root: &str,
        item_run: Option<&str>,
    ) -> Result<Vec<EventRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT e.at, e.kind, w.item_key AS item, s.step_key AS step, s.iteration, \
             x.attempt, e.execution_id AS execution, e.data FROM event e \
             LEFT JOIN workflow_run w ON w.id = e.workflow_run_id \
             LEFT JOIN step_run s ON s.id = e.step_run_id \
             LEFT JOIN execution x ON x.id = e.execution_id \
             WHERE e.root_run_id = ?1 AND CASE WHEN ?2 IS NULL \
               THEN e.workflow_run_id IS NULL OR e.workflow_run_id = ?1 OR e.kind LIKE 'run.%' \
               ELSE e.workflow_run_id = ?2 END \
             ORDER BY e.seq",
        )
        .bind(root)
        .bind(item_run)
        .fetch_all(&self.pool)
        .await?)
    }
}
