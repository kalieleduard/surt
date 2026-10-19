use serde_json::Value;

use super::{Store, StoreError};

impl Store {
    pub(crate) async fn record_runtime_info(
        &self,
        execution: &str,
        fields: &Value,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE execution SET runtime_info = \
             json_patch(coalesce(runtime_info, '{}'), ?1) WHERE id = ?2",
        )
        .bind(fields.to_string())
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        Ok(transaction.commit().await?)
    }
}
