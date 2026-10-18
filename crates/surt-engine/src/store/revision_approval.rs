use sqlx::FromRow;

use super::{Store, StoreError};

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ApprovalRow {
    pub(crate) decision: String,
    pub(crate) actor: String,
    pub(crate) decided_at: i64,
}

impl Store {
    pub(crate) async fn revision_approval(
        &self,
        revision: &str,
    ) -> Result<Option<ApprovalRow>, StoreError> {
        Ok(
            sqlx::query_as(
                "SELECT decision, actor, decided_at FROM approval WHERE revision_id = ?",
            )
            .bind(revision)
            .fetch_optional(&self.pool)
            .await?,
        )
    }
}
