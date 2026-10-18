use serde_json::json;

use super::graph::block_reason_name;
use super::runs::new_id;
use super::{
    About, Checkpoint, Store, StoreError, details, record_event, record_event_data, unix_now,
};
use crate::drive::RunEnd;
use crate::drive::graph::BlockReason;

const ITERATION_STARTED: &str = "iteration.started";
const STEP_RUNNING: &str = "step.running";

enum StepMove<'a> {
    Open,
    Skip,
    Block(BlockReason),
    Finish(&'a str),
}

impl StepMove<'_> {
    fn sql(&self) -> &'static str {
        match self {
            StepMove::Open => {
                "UPDATE step_run SET state = 'running', started_at = coalesce(started_at, ?1), outcome = ?3 \
                 WHERE id = ?2 AND state = 'pending'"
            }
            StepMove::Skip => {
                "UPDATE step_run SET state = 'skipped', finished_at = ?1, outcome = ?3 \
                 WHERE id = ?2 AND state = 'pending'"
            }
            StepMove::Block(_) => {
                "UPDATE step_run SET state = 'blocked', blocked_reason = ?3, started_at = coalesce(started_at, ?1) \
                 WHERE id = ?2 AND state = 'running'"
            }
            StepMove::Finish(_) => {
                "UPDATE step_run SET state = 'done', outcome = ?3, finished_at = ?1 \
                 WHERE id = ?2 AND state = 'running'"
            }
        }
    }

    fn value(&self) -> Option<&str> {
        match self {
            StepMove::Open | StepMove::Skip => None,
            StepMove::Block(reason) => Some(block_reason_name(*reason)),
            StepMove::Finish(outcome) => Some(outcome),
        }
    }

    fn details(&self) -> serde_json::Value {
        match self {
            StepMove::Open | StepMove::Skip => json!({}),
            StepMove::Block(reason) => json!({ "reason": block_reason_name(*reason) }),
            StepMove::Finish(outcome) => json!({ "outcome": outcome }),
        }
    }

    fn event(&self) -> &'static str {
        match self {
            StepMove::Open => STEP_RUNNING,
            StepMove::Skip => "step.skipped",
            StepMove::Block(_) => "step.blocked",
            StepMove::Finish(_) => "step.done",
        }
    }
}

impl Store {
    pub(crate) async fn start_iteration(
        &self,
        loop_step: &str,
        body: &[(&str, &str)],
        until: &str,
    ) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let (run, iteration): (String, i64) = sqlx::query_as(
            "SELECT workflow_run_id, (SELECT coalesce(max(iteration), 0) + 1 FROM step_run \
             WHERE loop_step_run_id = ?1) FROM step_run WHERE id = ?1",
        )
        .bind(loop_step)
        .fetch_one(&mut *transaction)
        .await?;
        for (key, kind) in body {
            sqlx::query(
                "INSERT INTO step_run (id, workflow_run_id, step_key, kind, loop_step_run_id, iteration, state, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, 'pending', ?)",
            )
            .bind(new_id())
            .bind(&run)
            .bind(key)
            .bind(kind)
            .bind(loop_step)
            .bind(iteration)
            .bind(now)
            .execute(&mut *transaction)
            .await?;
        }
        let opened = StepMove::Open;
        let moved = sqlx::query(opened.sql())
            .bind(now)
            .bind(loop_step)
            .bind(opened.value())
            .execute(&mut *transaction)
            .await?;
        if moved.rows_affected() > 0 {
            record_event(&mut transaction, About::Step(loop_step), STEP_RUNNING, now).await?;
        }
        let data =
            details::iteration_started(&mut transaction, loop_step, iteration, until).await?;
        let about = About::Step(loop_step);
        record_event_data(&mut transaction, about, ITERATION_STARTED, now, &data).await?;
        transaction.commit().await?;
        self.reached(Checkpoint::IterationOrExpansion)
    }

    pub(crate) async fn open_step(&self, step: &str) -> Result<(), StoreError> {
        self.move_step(step, StepMove::Open).await
    }

    pub(crate) async fn skip_step(&self, step: &str) -> Result<(), StoreError> {
        self.move_step(step, StepMove::Skip).await
    }

    pub(crate) async fn block_step(
        &self,
        step: &str,
        reason: BlockReason,
    ) -> Result<(), StoreError> {
        self.move_step(step, StepMove::Block(reason)).await
    }

    pub(crate) async fn finish_step(&self, step: &str, outcome: &str) -> Result<(), StoreError> {
        self.move_step(step, StepMove::Finish(outcome)).await
    }

    pub(crate) async fn finish_run(&self, run: &str, end: RunEnd) -> Result<(), StoreError> {
        let now = unix_now();
        let (state, event) = match end {
            RunEnd::Succeeded => ("succeeded", "run.succeeded"),
            RunEnd::Cancelled => ("cancelled", "run.cancelled"),
        };
        let mut transaction = self.begin_write().await?;
        let moved = sqlx::query(
            "UPDATE workflow_run SET state = ?, finished_at = ? WHERE id = ? AND state = 'active'",
        )
        .bind(state)
        .bind(now)
        .bind(run)
        .execute(&mut *transaction)
        .await?;
        if moved.rows_affected() > 0 {
            record_event(&mut transaction, About::Run(run), event, now).await?;
        }
        Ok(transaction.commit().await?)
    }

    async fn move_step(&self, step: &str, movement: StepMove<'_>) -> Result<(), StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let moved = sqlx::query(movement.sql())
            .bind(now)
            .bind(step)
            .bind(movement.value())
            .execute(&mut *transaction)
            .await?;
        if moved.rows_affected() > 0 {
            let data = movement.details();
            let about = About::Step(step);
            record_event_data(&mut transaction, about, movement.event(), now, &data).await?;
        }
        Ok(transaction.commit().await?)
    }
}
