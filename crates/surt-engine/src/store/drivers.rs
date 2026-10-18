use sqlx::FromRow;

use super::{About, Store, StoreError, record_event, unix_now};

const DRIVER_STARTED: &str = "driver.started";
const EXECUTION_INTERRUPTED: &str = "execution.interrupted";

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct DriverRecord {
    pub(crate) subject: String,
    pub(crate) pid: Option<i64>,
    pub(crate) since: Option<i64>,
}

impl Store {
    pub(crate) async fn record_driver(&self, root: &str, pid: u32) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE workflow_run SET driver_pid = ?, driver_since = ?, driver_start_time = NULL, \
             driver_boot_id = NULL WHERE id = ?",
        )
        .bind(i64::from(pid))
        .bind(now)
        .bind(root)
        .execute(&mut *transaction)
        .await?;
        record_event(&mut transaction, About::Run(root), DRIVER_STARTED, now).await?;
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn driver(&self, root: &str) -> Result<DriverRecord, StoreError> {
        Ok(sqlx::query_as(
            "SELECT s.name AS subject, w.driver_pid AS pid, w.driver_since AS since FROM workflow_run w \
             JOIN subject s ON s.id = w.subject_id WHERE w.id = ?",
        )
        .bind(root)
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn interrupt_stale(&self, root: &str) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let live: Vec<String> = sqlx::query_scalar(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? AND e.state IN ('queued', 'running')",
        )
        .bind(root)
        .fetch_all(&mut *transaction)
        .await?;
        for execution in &live {
            sqlx::query("UPDATE execution SET state = 'interrupted', finished_at = ? WHERE id = ?")
                .bind(now)
                .bind(execution)
                .execute(&mut *transaction)
                .await?;
            record_event(
                &mut transaction,
                About::Execution(execution),
                EXECUTION_INTERRUPTED,
                now,
            )
            .await?;
        }
        Ok(transaction.commit().await?)
    }
}
