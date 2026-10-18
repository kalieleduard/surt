use sqlx::SqliteConnection;

use super::graph::failure_class_name;
use super::revisions::number;
use super::runs::new_id;
use super::{About, Store, StoreError, record_event, unix_now};
use crate::runtime::FailureClass;

const STEP_RUNNING: &str = "step.running";

pub(crate) struct ExecutionStart<'a> {
    pub(crate) step: &'a str,
    pub(crate) agent_snapshot: &'a str,
    pub(crate) inputs: Vec<InputRequest>,
}

pub(crate) struct InputRequest {
    pub(crate) input: String,
    pub(crate) artifact: String,
}

#[derive(Clone, Debug)]
pub(crate) struct StartedExecution {
    pub(crate) id: String,
    pub(crate) attempt: u32,
    pub(crate) inputs: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub(crate) enum ExecutionEnd {
    Failed {
        class: FailureClass,
        code: String,
        message: String,
    },
    Interrupted,
}

impl Store {
    pub(crate) async fn start_execution(
        &self,
        start: ExecutionStart<'_>,
    ) -> Result<StartedExecution, StoreError> {
        let now = unix_now();
        let mut transaction = self.begin_write().await?;
        let (run, attempt): (String, i64) = sqlx::query_as(
            "SELECT workflow_run_id, (SELECT coalesce(max(attempt), 0) + 1 FROM execution \
             WHERE step_run_id = ?1) FROM step_run WHERE id = ?1",
        )
        .bind(start.step)
        .fetch_one(&mut *transaction)
        .await?;
        let opened = sqlx::query(
            "UPDATE step_run SET state = 'running', started_at = coalesce(started_at, ?) \
             WHERE id = ? AND state = 'pending'",
        )
        .bind(now)
        .bind(start.step)
        .execute(&mut *transaction)
        .await?;
        if opened.rows_affected() > 0 {
            record_event(&mut transaction, About::Step(start.step), STEP_RUNNING, now).await?;
        }
        let id = new_id();
        sqlx::query(
            "INSERT INTO execution (id, step_run_id, attempt, kind, agent_snapshot_sha256, state, started_at) \
             VALUES (?, ?, ?, 'agent', ?, 'running', ?)",
        )
        .bind(&id)
        .bind(start.step)
        .bind(attempt)
        .bind(start.agent_snapshot)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        record_event(
            &mut transaction,
            About::Execution(&id),
            "execution.started",
            now,
        )
        .await?;
        let mut inputs = Vec::new();
        for request in &start.inputs {
            if let Some(revision) = fix_input(&mut transaction, &run, &id, request, now).await? {
                inputs.push((request.input.clone(), revision));
            }
        }
        transaction.commit().await?;
        Ok(StartedExecution {
            id,
            attempt: number(attempt),
            inputs,
        })
    }

    pub(crate) async fn end_execution(
        &self,
        execution: &str,
        end: &ExecutionEnd,
    ) -> Result<(), StoreError> {
        let now = unix_now();
        let (state, class, code, message, event) = match end {
            ExecutionEnd::Failed {
                class,
                code,
                message,
            } => (
                "failed",
                Some(failure_class_name(*class)),
                Some(code.as_str()),
                Some(message.as_str()),
                "execution.failed",
            ),
            ExecutionEnd::Interrupted => ("interrupted", None, None, None, "execution.interrupted"),
        };
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE execution SET state = ?, failure_class = ?, error_code = ?, error_message = ?, \
             finished_at = ? WHERE id = ? AND state IN ('queued', 'running')",
        )
        .bind(state)
        .bind(class)
        .bind(code)
        .bind(message)
        .bind(now)
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        record_event(&mut transaction, About::Execution(execution), event, now).await?;
        Ok(transaction.commit().await?)
    }
}

async fn fix_input(
    connection: &mut SqliteConnection,
    run: &str,
    execution: &str,
    request: &InputRequest,
    now: i64,
) -> Result<Option<String>, sqlx::Error> {
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
         WHERE a.workflow_run_id = ? AND a.name = ? ORDER BY r.number DESC LIMIT 1",
    )
    .bind(run)
    .bind(&request.artifact)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(revision) = latest else {
        return Ok(None);
    };
    sqlx::query(
        "INSERT INTO execution_input (execution_id, input_name, revision_id) VALUES (?, ?, ?)",
    )
    .bind(execution)
    .bind(&request.input)
    .bind(&revision)
    .execute(&mut *connection)
    .await?;
    let pinned = sqlx::query(
        "UPDATE artifact_revision SET pinned_at = ?1 WHERE id = ?2 AND pinned_at IS NULL \
         AND EXISTS (SELECT 1 FROM approval WHERE revision_id = ?2 AND decision = 'approve')",
    )
    .bind(now)
    .bind(&revision)
    .execute(&mut *connection)
    .await?;
    if pinned.rows_affected() > 0 {
        record_event(
            connection,
            About::Execution(execution),
            "revision.pinned",
            now,
        )
        .await?;
    }
    Ok(Some(revision))
}
