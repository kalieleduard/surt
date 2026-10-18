use sqlx::SqliteConnection;

use super::graph::failure_class_name;
use super::revisions::number;
use super::runs::new_id;
use super::{About, Checkpoint, Store, StoreError, record_event, unix_now};
use crate::runtime::FailureClass;

const STEP_RUNNING: &str = "step.running";

pub(crate) struct ExecutionStart<'a> {
    pub(crate) step: &'a str,
    pub(crate) kind: ExecutionKind<'a>,
    pub(crate) inputs: Vec<InputRequest>,
}

pub(crate) enum ExecutionKind<'a> {
    Agent { snapshot: &'a str, slot: usize },
    Operation { id: &'a str },
}

pub(crate) struct InputRequest {
    pub(crate) input: String,
    pub(crate) artifact: String,
    pub(crate) run: String,
    pub(crate) item: Option<String>,
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
        resets_at: Option<i64>,
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
        let attempt: i64 = sqlx::query_scalar(
            "SELECT coalesce(max(attempt), 0) + 1 FROM execution WHERE step_run_id = ?",
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
        let (kind, snapshot, operation, slot) = match start.kind {
            ExecutionKind::Agent { snapshot, slot } => (
                "agent",
                Some(snapshot),
                None,
                Some(i64::try_from(slot).unwrap_or(i64::MAX)),
            ),
            ExecutionKind::Operation { id } => ("operation", None, Some(id), None),
        };
        sqlx::query(
            "INSERT INTO execution (id, step_run_id, attempt, kind, agent_snapshot_sha256, operation_id, state, slot, started_at) \
             VALUES (?, ?, ?, ?, ?, ?, 'running', ?, ?)",
        )
        .bind(&id)
        .bind(start.step)
        .bind(attempt)
        .bind(kind)
        .bind(snapshot)
        .bind(operation)
        .bind(slot)
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
            if let Some(revision) = fix_input(&mut transaction, &id, request, now).await? {
                inputs.push((request.input.clone(), revision));
            }
        }
        transaction.commit().await?;
        self.reached(Checkpoint::ExecutionStarted)?;
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
        let (state, class, code, message, resets_at, event) = match end {
            ExecutionEnd::Failed {
                class,
                code,
                message,
                resets_at,
            } => (
                "failed",
                Some(failure_class_name(*class)),
                Some(code.as_str()),
                Some(message.as_str()),
                *resets_at,
                "execution.failed",
            ),
            ExecutionEnd::Interrupted => (
                "interrupted",
                None,
                None,
                None,
                None,
                "execution.interrupted",
            ),
        };
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "UPDATE execution SET state = ?1, failure_class = ?2, error_code = ?3, error_message = ?4, \
             runtime_info = CASE WHEN ?5 IS NULL THEN runtime_info \
             ELSE json_set(coalesce(runtime_info, '{}'), '$.resets_at', ?5) END, \
             finished_at = ?6 WHERE id = ?7 AND state IN ('queued', 'running')",
        )
        .bind(state)
        .bind(class)
        .bind(code)
        .bind(message)
        .bind(resets_at)
        .bind(now)
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        record_event(&mut transaction, About::Execution(execution), event, now).await?;
        transaction.commit().await?;
        self.reached(Checkpoint::ExecutionEnded)
    }
}

async fn fix_input(
    connection: &mut SqliteConnection,
    execution: &str,
    request: &InputRequest,
    now: i64,
) -> Result<Option<String>, sqlx::Error> {
    let latest: Option<String> = sqlx::query_scalar(
        "SELECT r.id FROM artifact a JOIN artifact_revision r ON r.artifact_id = a.id \
         WHERE a.workflow_run_id = ? AND a.name = ? AND (? IS NULL OR r.pinned_at IS NOT NULL) \
         ORDER BY r.number DESC LIMIT 1",
    )
    .bind(&request.run)
    .bind(&request.artifact)
    .bind(&request.item)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(revision) = latest else {
        return Ok(None);
    };
    sqlx::query(
        "INSERT INTO execution_input (execution_id, input_name, revision_id, item_key) VALUES (?, ?, ?, ?)",
    )
    .bind(execution)
    .bind(&request.input)
    .bind(&revision)
    .bind(&request.item)
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
