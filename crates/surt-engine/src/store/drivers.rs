use sqlx::FromRow;

use super::revisions::number;
use super::{About, Checkpoint, Store, StoreError, record_event, unix_now};
use crate::drive::StepLabel;
use crate::drive::graph::RunId;
use crate::platform::ProcessIdentity;

const DRIVER_STARTED: &str = "driver.started";

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct DriverRecord {
    pub(crate) subject: String,
    pub(crate) pid: Option<i64>,
    pub(crate) since: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StaleExecution {
    pub(crate) id: String,
    pub(crate) attempt: u32,
    pub(crate) running: bool,
    pub(crate) step: StepLabel,
    pub(crate) identity: Option<ProcessIdentity>,
}

#[derive(FromRow)]
struct StaleRow {
    id: String,
    attempt: i64,
    state: String,
    pid: Option<i64>,
    process_start_time: Option<i64>,
    process_boot_id: Option<String>,
    step_key: String,
    item_key: Option<String>,
    iteration: Option<i64>,
}

impl Store {
    pub(crate) async fn record_driver(
        &self,
        root: &RunId,
        pid: u32,
        identity: Option<&ProcessIdentity>,
    ) -> Result<(), StoreError> {
        let root = root.as_str();
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE workflow_run SET driver_pid = ?, driver_since = ?, driver_start_time = ?, \
             driver_boot_id = ? WHERE id = ?",
        )
        .bind(i64::from(pid))
        .bind(now)
        .bind(identity.map(|identity| identity.start_time))
        .bind(identity.map(|identity| identity.boot_id.as_str()))
        .bind(root)
        .execute(&mut *transaction)
        .await?;
        record_event(&mut transaction, About::Run(root), DRIVER_STARTED, now).await?;
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn driver(&self, root: &RunId) -> Result<DriverRecord, StoreError> {
        let root = root.as_str();
        Ok(sqlx::query_as(
            "SELECT s.name AS subject, w.driver_pid AS pid, w.driver_since AS since FROM workflow_run w \
             JOIN subject s ON s.id = w.subject_id WHERE w.id = ?",
        )
        .bind(root)
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn record_process(
        &self,
        execution: &str,
        identity: &ProcessIdentity,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE execution SET pid = ?, process_start_time = ?, process_boot_id = ? \
             WHERE id = ? AND state IN ('queued', 'running')",
        )
        .bind(i64::from(identity.pid))
        .bind(identity.start_time)
        .bind(&identity.boot_id)
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.reached(Checkpoint::ProcessSpawned)
    }

    pub(crate) async fn stale_executions(
        &self,
        root: &str,
    ) -> Result<Vec<StaleExecution>, StoreError> {
        let rows: Vec<StaleRow> = sqlx::query_as(
            "SELECT e.id, e.attempt, e.state, e.pid, e.process_start_time, e.process_boot_id, \
             s.step_key, w.item_key, s.iteration FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? AND e.state IN ('queued', 'running') \
             ORDER BY e.started_at, e.id",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(StaleExecution::from).collect())
    }
}

impl From<StaleRow> for StaleExecution {
    fn from(row: StaleRow) -> StaleExecution {
        let identity = match (row.pid, row.process_start_time, row.process_boot_id) {
            (Some(pid), Some(start_time), Some(boot_id)) => {
                u32::try_from(pid).ok().map(|pid| ProcessIdentity {
                    pid,
                    start_time,
                    boot_id,
                })
            }
            _ => None,
        };
        StaleExecution {
            id: row.id,
            attempt: number(row.attempt),
            running: row.state == "running",
            step: StepLabel {
                key: row.step_key,
                item: row.item_key,
                iteration: row.iteration.map(number),
            },
            identity,
        }
    }
}
