use sqlx::FromRow;

use super::{About, Store, StoreError, record_event, unix_now};

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct RefMove {
    pub(crate) ref_name: String,
    pub(crate) old_sha: String,
    pub(crate) new_sha: String,
}

impl Store {
    pub(crate) async fn ref_move(&self, step: &str) -> Result<Option<RefMove>, StoreError> {
        Ok(
            sqlx::query_as("SELECT ref_name, old_sha, new_sha FROM ref_move WHERE step_run_id = ?")
                .bind(step)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    pub(crate) async fn intend_ref_move(
        &self,
        step: &str,
        intended: &RefMove,
    ) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "INSERT INTO ref_move (step_run_id, ref_name, old_sha, new_sha, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(step)
        .bind(&intended.ref_name)
        .bind(&intended.old_sha)
        .bind(&intended.new_sha)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        record_event(
            &mut transaction,
            About::Step(step),
            "ref_move.intended",
            now,
        )
        .await?;
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn forget_ref_move(&self, step: &str) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query("DELETE FROM ref_move WHERE step_run_id = ?")
            .bind(step)
            .execute(&mut *transaction)
            .await?;
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn merged_children(
        &self,
        run: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT w.item_key, b.merged_as_sha FROM workflow_run w \
             JOIN run_branch b ON b.workflow_run_id = w.id \
             JOIN step_run p ON p.id = w.parent_step_run_id \
             WHERE p.workflow_run_id = ? AND b.merged_as_sha IS NOT NULL ORDER BY w.item_key",
        )
        .bind(run)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn step_started_at(&self, step: &str) -> Result<Option<i64>, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT started_at FROM step_run WHERE id = ?")
                .bind(step)
                .fetch_one(&self.pool)
                .await?,
        )
    }
}
