use sqlx::SqliteConnection;

use super::{About, Store, StoreError, record_event, record_event_data, unix_now};
use crate::drive::graph::RunId;

impl Store {
    pub(crate) async fn unblock(&self, root: &RunId) -> Result<(), StoreError> {
        let root = root.as_str();
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let blocked = ids(
            &mut transaction,
            "SELECT id FROM step_run WHERE state = 'blocked' \
             AND workflow_run_id IN (SELECT id FROM workflow_run WHERE root_run_id = ?) \
             ORDER BY created_at, id",
            root,
        )
        .await?;
        for step in &blocked {
            let base: i64 = sqlx::query_scalar(
                "UPDATE step_run SET state = 'running', blocked_reason = NULL, budget_base = CASE kind \
                 WHEN 'loop' THEN (SELECT coalesce(max(iteration), 0) FROM step_run WHERE loop_step_run_id = ?1) \
                 ELSE (SELECT count(*) FROM execution WHERE step_run_id = ?1) END WHERE id = ?1 \
                 RETURNING budget_base",
            )
            .bind(step)
            .fetch_one(&mut *transaction)
            .await?;
            let data = serde_json::json!({ "budget_base": base });
            let about = About::Step(step);
            record_event_data(&mut transaction, about, "step.unblocked", now, &data).await?;
        }
        Ok(transaction.commit().await?)
    }

    pub(crate) async fn cancel_run(&self, root: &RunId) -> Result<(), StoreError> {
        let root = root.as_str();
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let executions = ids(
            &mut transaction,
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE e.state IN ('queued', 'running') \
             AND s.workflow_run_id IN (SELECT id FROM workflow_run WHERE root_run_id = ?)",
            root,
        )
        .await?;
        for execution in &executions {
            sqlx::query("UPDATE execution SET state = 'cancelled', finished_at = ? WHERE id = ?")
                .bind(now)
                .bind(execution)
                .execute(&mut *transaction)
                .await?;
            let about = About::Execution(execution);
            record_event(&mut transaction, about, "execution.cancelled", now).await?;
        }
        let steps = ids(
            &mut transaction,
            "SELECT id FROM step_run WHERE state IN ('pending', 'running', 'blocked') \
             AND workflow_run_id IN (SELECT id FROM workflow_run WHERE root_run_id = ?)",
            root,
        )
        .await?;
        for step in &steps {
            sqlx::query(
                "UPDATE step_run SET state = 'cancelled', blocked_reason = NULL, finished_at = ? WHERE id = ?",
            )
            .bind(now)
            .bind(step)
            .execute(&mut *transaction)
            .await?;
            record_event(&mut transaction, About::Step(step), "step.cancelled", now).await?;
        }
        let runs = ids(
            &mut transaction,
            "SELECT id FROM workflow_run WHERE root_run_id = ? AND state = 'active' \
             ORDER BY parent_step_run_id IS NULL, created_at, id",
            root,
        )
        .await?;
        for run in &runs {
            sqlx::query(
                "UPDATE workflow_run SET state = 'cancelled', finished_at = ? WHERE id = ?",
            )
            .bind(now)
            .bind(run)
            .execute(&mut *transaction)
            .await?;
            record_event(&mut transaction, About::Run(run), "run.cancelled", now).await?;
        }
        Ok(transaction.commit().await?)
    }
}

async fn ids(
    connection: &mut SqliteConnection,
    sql: &'static str,
    root: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(sql)
        .bind(root)
        .fetch_all(connection)
        .await
}
