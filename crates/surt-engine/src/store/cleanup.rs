use std::path::PathBuf;

use sqlx::FromRow;

use super::{Store, StoreError, unix_now};
use crate::drive::RunEnd;

const FINISHED: &str = "cleanup.finished";
const INCOMPLETE: &str = "cleanup.incomplete";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ending {
    pub(crate) end: RunEnd,
    pub(crate) cleaned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChildBranch {
    pub(crate) run: String,
    pub(crate) ref_name: String,
    pub(crate) tip_sha: String,
    pub(crate) worktree_path: Option<PathBuf>,
    pub(crate) last_execution: Option<String>,
}

#[derive(FromRow)]
struct EndingRow {
    state: String,
    cleaned: bool,
}

#[derive(FromRow)]
struct ChildBranchRow {
    run: String,
    ref_name: String,
    tip_sha: String,
    worktree_path: Option<String>,
    last_execution: Option<String>,
}

pub(crate) enum CleanupRecord {
    Finished(serde_json::Value),
    Incomplete(serde_json::Value),
}

impl Store {
    pub(crate) async fn ending(&self, root: &str) -> Result<Option<Ending>, StoreError> {
        let row: Option<EndingRow> = sqlx::query_as(
            "SELECT state, cleaned_at IS NOT NULL AS cleaned FROM workflow_run \
             WHERE id = ? AND state <> 'active'",
        )
        .bind(root)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| Ending {
            end: if row.state == "succeeded" {
                RunEnd::Succeeded
            } else {
                RunEnd::Cancelled
            },
            cleaned: row.cleaned,
        }))
    }

    pub(crate) async fn execution_ids(&self, root: &str) -> Result<Vec<String>, StoreError> {
        Ok(sqlx::query_scalar(
            "SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE w.root_run_id = ? ORDER BY e.id",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn child_branches(&self, root: &str) -> Result<Vec<ChildBranch>, StoreError> {
        let rows: Vec<ChildBranchRow> = sqlx::query_as(
            "SELECT b.workflow_run_id AS run, b.ref_name, b.tip_sha, b.worktree_path, \
             (SELECT e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
              WHERE s.workflow_run_id = b.workflow_run_id \
              ORDER BY e.started_at DESC, e.id DESC LIMIT 1) AS last_execution \
             FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
             WHERE w.root_run_id = ? AND w.parent_step_run_id IS NOT NULL ORDER BY w.item_key",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| ChildBranch {
                run: row.run,
                ref_name: row.ref_name,
                tip_sha: row.tip_sha,
                worktree_path: row.worktree_path.map(PathBuf::from),
                last_execution: row.last_execution,
            })
            .collect())
    }

    pub(crate) async fn record_cleanup(
        &self,
        root: &str,
        record: CleanupRecord,
    ) -> Result<(), StoreError> {
        let now = unix_now();
        let (kind, data, complete) = match record {
            CleanupRecord::Finished(data) => (FINISHED, data, true),
            CleanupRecord::Incomplete(data) => (INCOMPLETE, data, false),
        };
        let mut transaction = self.begin_write().await?;
        if complete {
            sqlx::query("UPDATE workflow_run SET cleaned_at = ? WHERE id = ?")
                .bind(now)
                .bind(root)
                .execute(&mut *transaction)
                .await?;
        }
        sqlx::query(
            "INSERT INTO event (at, root_run_id, workflow_run_id, kind, data) VALUES (?1, ?2, ?2, ?3, ?4)",
        )
        .bind(now)
        .bind(root)
        .bind(kind)
        .bind(data.to_string())
        .execute(&mut *transaction)
        .await?;
        Ok(transaction.commit().await?)
    }
}
