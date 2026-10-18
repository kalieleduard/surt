use serde_json::json;
use sqlx::{FromRow, SqliteConnection};

use super::columns::number;
use super::{About, Checkpoint, Store, StoreError, record_event_data, unix_now};
use crate::drive::StepLabel;

pub(super) const CONFIRMED: &str = "intention.confirmed";
const DISCARDED: &str = "intention.discarded";
const INTENDED: &str = "ref_move.intended";

pub(super) async fn settle_ref_move(
    connection: &mut SqliteConnection,
    step: &str,
    kind: &str,
    now: i64,
) -> Result<(), sqlx::Error> {
    let settled: Option<RefMove> = sqlx::query_as(
        "DELETE FROM ref_move WHERE step_run_id = ? RETURNING ref_name, old_sha, new_sha",
    )
    .bind(step)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(settled) = settled else {
        return Ok(());
    };
    let data = details(&settled);
    record_event_data(connection, About::Step(step), kind, now, &data).await
}

fn details(intended: &RefMove) -> serde_json::Value {
    json!({
        "intention": "ref_move",
        "ref": intended.ref_name,
        "old_sha": intended.old_sha,
        "new_sha": intended.new_sha,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub(crate) struct RefMove {
    pub(crate) ref_name: String,
    pub(crate) old_sha: String,
    pub(crate) new_sha: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PendingRefMove {
    pub(crate) step_run: String,
    pub(crate) step: StepLabel,
    pub(crate) operation: Option<String>,
    pub(crate) intended: RefMove,
}

#[derive(FromRow)]
struct PendingRow {
    step_run_id: String,
    step_key: String,
    item_key: Option<String>,
    iteration: Option<i64>,
    operation_id: Option<String>,
    ref_name: String,
    old_sha: String,
    new_sha: String,
}

impl Store {
    pub(crate) async fn pending_ref_moves(
        &self,
        root: &str,
    ) -> Result<Vec<PendingRefMove>, StoreError> {
        let rows: Vec<PendingRow> = sqlx::query_as(
            "SELECT m.step_run_id, s.step_key, w.item_key, s.iteration, \
             (SELECT e.operation_id FROM execution e WHERE e.step_run_id = m.step_run_id \
              ORDER BY e.attempt DESC LIMIT 1) AS operation_id, m.ref_name, m.old_sha, m.new_sha \
             FROM ref_move m JOIN step_run s ON s.id = m.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE w.root_run_id = ? ORDER BY m.created_at, m.step_run_id",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| PendingRefMove {
                step_run: row.step_run_id,
                step: StepLabel {
                    key: row.step_key,
                    item: row.item_key,
                    iteration: row.iteration.map(number),
                },
                operation: row.operation_id,
                intended: RefMove {
                    ref_name: row.ref_name,
                    old_sha: row.old_sha,
                    new_sha: row.new_sha,
                },
            })
            .collect())
    }

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
        let data = details(intended);
        record_event_data(&mut transaction, About::Step(step), INTENDED, now, &data).await?;
        transaction.commit().await?;
        self.reached(Checkpoint::Intention)
    }

    pub(crate) async fn forget_ref_move(&self, step: &str) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        settle_ref_move(&mut transaction, step, DISCARDED, unix_now()).await?;
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
