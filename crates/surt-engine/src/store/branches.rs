use std::path::PathBuf;

use sqlx::FromRow;

use super::graph::execution_state;
use super::{Checkpoint, Store, StoreError, unix_now};
use crate::drive::graph::ExecutionState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunBranch {
    pub(crate) ref_name: String,
    pub(crate) start_sha: String,
    pub(crate) tip_sha: String,
    pub(crate) worktree_path: Option<PathBuf>,
}

#[derive(FromRow)]
struct BranchRow {
    ref_name: String,
    start_sha: String,
    tip_sha: String,
    worktree_path: Option<String>,
}

#[derive(FromRow)]
struct PreviousRow {
    id: String,
    state: String,
    failure_class: Option<String>,
}

pub(crate) struct PreviousExecution {
    pub(crate) id: String,
    pub(crate) state: ExecutionState,
}

impl Store {
    pub(crate) async fn run_branch(&self, run: &str) -> Result<Option<RunBranch>, StoreError> {
        let row: Option<BranchRow> = sqlx::query_as(
            "SELECT ref_name, start_sha, tip_sha, worktree_path FROM run_branch \
             WHERE workflow_run_id = ?",
        )
        .bind(run)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| RunBranch {
            ref_name: row.ref_name,
            start_sha: row.start_sha,
            tip_sha: row.tip_sha,
            worktree_path: row.worktree_path.map(PathBuf::from),
        }))
    }

    pub(crate) async fn record_run_branch(
        &self,
        run: &str,
        branch: &RunBranch,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "INSERT INTO run_branch (workflow_run_id, ref_name, start_sha, tip_sha, worktree_path, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(run)
        .bind(&branch.ref_name)
        .bind(&branch.start_sha)
        .bind(&branch.tip_sha)
        .bind(
            branch
                .worktree_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        )
        .bind(unix_now())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.reached(Checkpoint::Intention)
    }

    pub(crate) async fn previous_execution(
        &self,
        run: &str,
        current: &str,
    ) -> Result<Option<PreviousExecution>, StoreError> {
        let row: Option<PreviousRow> = sqlx::query_as(
            "SELECT e.id, e.state, e.failure_class FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.workflow_run_id = ? AND e.id <> ? ORDER BY e.started_at DESC, e.id DESC LIMIT 1",
        )
        .bind(run)
        .bind(current)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| PreviousExecution {
            state: execution_state(&row.state, row.failure_class.as_deref()),
            id: row.id,
        }))
    }

    pub(crate) async fn operation_succeeded(
        &self,
        run: &str,
        operation: &str,
    ) -> Result<bool, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.workflow_run_id = ? AND e.operation_id = ? AND e.state = 'succeeded')",
        )
        .bind(run)
        .bind(operation)
        .fetch_one(&self.pool)
        .await?)
    }

    pub(crate) async fn record_execution_event(
        &self,
        execution: &str,
        kind: &str,
        data: &serde_json::Value,
    ) -> Result<(), StoreError> {
        let mut transaction = self.begin_write().await?;
        sqlx::query(
            "INSERT INTO event (at, root_run_id, workflow_run_id, step_run_id, execution_id, kind, data) \
             SELECT ?1, w.root_run_id, w.id, s.id, e.id, ?2, ?3 FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE e.id = ?4",
        )
        .bind(unix_now())
        .bind(kind)
        .bind(data.to_string())
        .bind(execution)
        .execute(&mut *transaction)
        .await?;
        Ok(transaction.commit().await?)
    }
}
