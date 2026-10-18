use super::{Store, StoreError};

impl Store {
    pub(crate) async fn record_runtime_version(
        &self,
        execution: &str,
        version: &str,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE execution SET runtime_info = \
             json_set(coalesce(runtime_info, '{}'), '$.runtime_version', ?1) WHERE id = ?2",
        )
        .bind(version)
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        Ok(transaction.commit().await?)
    }
}
