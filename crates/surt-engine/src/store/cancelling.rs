use sqlx::FromRow;

use super::{About, Store, StoreError, record_event, unix_now};

#[derive(FromRow)]
pub(crate) struct ImpactRow {
    pub(crate) executions: i64,
    pub(crate) items: i64,
    pub(crate) merged: i64,
}

impl Store {
    pub(crate) async fn request_cancel(&self, root: &str, actor: &str) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let requested = sqlx::query(
            "UPDATE workflow_run SET cancel_requested_at = ?, cancel_requested_by = ? \
             WHERE id = ? AND state = 'active' AND cancel_requested_at IS NULL",
        )
        .bind(now)
        .bind(actor)
        .bind(root)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if requested > 0 {
            record_event(
                &mut transaction,
                About::Run(root),
                "run.cancel_requested",
                now,
            )
            .await?;
        }
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn cancel_impact(&self, root: &str) -> Result<ImpactRow, StoreError> {
        Ok(sqlx::query_as(
            "SELECT \
             (SELECT count(*) FROM execution e JOIN step_run s ON s.id = e.step_run_id \
              JOIN workflow_run w ON w.id = s.workflow_run_id \
              WHERE w.root_run_id = ?1 AND e.state IN ('queued', 'running')) AS executions, \
             (SELECT count(*) FROM workflow_run \
              WHERE root_run_id = ?1 AND parent_step_run_id IS NOT NULL) AS items, \
             (SELECT count(*) FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
              WHERE w.root_run_id = ?1 AND w.parent_step_run_id IS NOT NULL \
              AND b.merged_as_sha IS NOT NULL) AS merged",
        )
        .bind(root)
        .fetch_one(&self.pool)
        .await?)
    }
}
