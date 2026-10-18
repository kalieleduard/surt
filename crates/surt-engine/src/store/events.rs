use serde_json::Value;

use super::{About, Store, StoreError, record_event_data, unix_now};

impl Store {
    pub(crate) async fn record_run_event(
        &self,
        run: &str,
        kind: &str,
        data: &Value,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        record_event_data(&mut transaction, About::Run(run), kind, unix_now(), data).await?;
        Ok(transaction.commit().await?)
    }
}
