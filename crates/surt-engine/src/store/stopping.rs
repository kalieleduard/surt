use super::{About, Store, StoreError, record_event_data, unix_now};

const STOP_REQUESTED: &str = "run.stop_requested";

impl Store {
    pub(crate) async fn request_stop(&self, root: &str, actor: &str) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let requested = sqlx::query(
            "UPDATE workflow_run SET stop_requested_at = ?, stop_requested_by = ? \
             WHERE id = ? AND state = 'active' AND stop_requested_at IS NULL",
        )
        .bind(now)
        .bind(actor)
        .bind(root)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
        if requested > 0 {
            let data = serde_json::json!({ "actor": actor });
            let about = About::Run(root);
            record_event_data(&mut transaction, about, STOP_REQUESTED, now, &data).await?;
        }
        Ok(transaction.commit().await?)
    }
}
