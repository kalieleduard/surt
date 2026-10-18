use sqlx::FromRow;

use super::{Store, StoreError};

const AGENT_FILE: &str = "agent.toml";

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ExecutionScopeRow {
    pub(crate) id: String,
    pub(crate) subject: String,
    pub(crate) ordinal: i64,
    pub(crate) item: Option<String>,
    pub(crate) step: String,
    pub(crate) iteration: Option<i64>,
    pub(crate) attempt: i64,
    pub(crate) state: String,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ExecutionDetailRow {
    #[sqlx(flatten)]
    pub(crate) scope: ExecutionScopeRow,
    pub(crate) role: Option<String>,
    pub(crate) contract: Option<Vec<u8>>,
    pub(crate) operation: Option<String>,
    pub(crate) outcome: Option<String>,
    pub(crate) failure_class: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) error_message: Option<String>,
    pub(crate) slot: Option<i64>,
    pub(crate) pid: Option<i64>,
    pub(crate) runtime_info: Option<String>,
    pub(crate) started_at: Option<i64>,
    pub(crate) finished_at: Option<i64>,
    pub(crate) worktree: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
pub(crate) struct ExecutionInputRow {
    pub(crate) name: String,
    pub(crate) artifact: String,
    pub(crate) revision: i64,
    pub(crate) sha256: String,
    pub(crate) item: Option<String>,
}

impl Store {
    pub(crate) async fn executions_starting_with(
        &self,
        prefix: &str,
    ) -> Result<Vec<ExecutionScopeRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT x.id, sub.name AS subject, root.ordinal, w.item_key AS item, \
             s.step_key AS step, s.iteration, x.attempt, x.state \
             FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             JOIN workflow_run root ON root.id = w.root_run_id \
             JOIN subject sub ON sub.id = root.subject_id \
             WHERE substr(x.id, 1, length(?1)) = ?1 ORDER BY x.id",
        )
        .bind(prefix)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn execution_detail(
        &self,
        id: &str,
    ) -> Result<Option<ExecutionDetailRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT x.id, sub.name AS subject, root.ordinal, w.item_key AS item, \
             s.step_key AS step, s.iteration, x.attempt, x.state, snap.name AS role, \
             b.bytes AS contract, x.operation_id AS operation, x.outcome, x.failure_class, \
             x.error_code, x.error_message, x.slot, x.pid, x.runtime_info, x.started_at, \
             x.finished_at, rb.worktree_path AS worktree \
             FROM execution x JOIN step_run s ON s.id = x.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             JOIN workflow_run root ON root.id = w.root_run_id \
             JOIN subject sub ON sub.id = root.subject_id \
             LEFT JOIN snapshot snap ON snap.sha256 = x.agent_snapshot_sha256 \
             LEFT JOIN snapshot_file f ON f.snapshot_sha256 = snap.sha256 AND f.rel_path = ?2 \
             LEFT JOIN blob b ON b.sha256 = f.blob_sha256 \
             LEFT JOIN run_branch rb ON rb.workflow_run_id = w.id \
             WHERE x.id = ?1",
        )
        .bind(id)
        .bind(AGENT_FILE)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn execution_inputs(
        &self,
        id: &str,
    ) -> Result<Vec<ExecutionInputRow>, StoreError> {
        Ok(sqlx::query_as(
            "SELECT i.input_name AS name, a.name AS artifact, r.number AS revision, \
             r.identity_sha256 AS sha256, i.item_key AS item FROM execution_input i \
             JOIN artifact_revision r ON r.id = i.revision_id \
             JOIN artifact a ON a.id = r.artifact_id \
             WHERE i.execution_id = ? ORDER BY i.input_name",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?)
    }
}
