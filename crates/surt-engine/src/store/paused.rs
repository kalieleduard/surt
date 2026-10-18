use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct LastAgentEnd {
    pub(crate) failure_class: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) resets_at: Option<i64>,
    pub(crate) finished_at: i64,
    pub(crate) started_after: bool,
}

impl Store {
    pub(crate) async fn last_agent_end(
        &self,
        root: &str,
    ) -> Result<Option<LastAgentEnd>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT e.failure_class, e.error_code, \
             json_extract(e.runtime_info, '$.resets_at') AS resets_at, e.finished_at, \
             EXISTS (SELECT 1 FROM execution x JOIN step_run t ON t.id = x.step_run_id \
               JOIN workflow_run v ON v.id = t.workflow_run_id \
               WHERE v.root_run_id = ?1 AND x.kind = 'agent' AND x.started_at > e.finished_at) \
               AS started_after \
             FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ?1 AND e.kind = 'agent' AND e.finished_at IS NOT NULL \
             ORDER BY e.finished_at DESC, e.rowid DESC LIMIT 1",
        )
        .bind(root)
        .fetch_optional(&self.pool)
        .await?)
    }
}
