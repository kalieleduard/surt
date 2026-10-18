use sqlx::FromRow;

use super::columns::number;
use super::graph::block_reason;
use super::{Store, StoreError};
use crate::drive::graph::BlockReason;

pub(crate) struct BlockedRow {
    pub(crate) key: String,
    pub(crate) item: Option<String>,
    pub(crate) iteration: Option<u32>,
    pub(crate) reason: BlockReason,
    pub(crate) failure: Option<FailureRow>,
}

pub(crate) struct FailureRow {
    pub(crate) execution: String,
    pub(crate) attempt: u32,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) resets_at: Option<i64>,
}

#[derive(FromRow)]
struct RawBlocked {
    step_key: String,
    item_key: Option<String>,
    iteration: Option<i64>,
    blocked_reason: String,
    execution_id: Option<String>,
    attempt: Option<i64>,
    error_code: Option<String>,
    error_message: Option<String>,
    resets_at: Option<i64>,
}

impl Store {
    pub(crate) async fn blocked_steps(&self, root: &str) -> Result<Vec<BlockedRow>, StoreError> {
        let rows: Vec<RawBlocked> = sqlx::query_as(
            "SELECT s.step_key, w.item_key, s.iteration, s.blocked_reason, e.id AS execution_id, \
             e.attempt, e.error_code, e.error_message, \
             json_extract(e.runtime_info, '$.resets_at') AS resets_at \
             FROM step_run s JOIN workflow_run w ON w.id = s.workflow_run_id \
             LEFT JOIN execution e ON e.id = ( \
               SELECT f.id FROM execution f WHERE f.step_run_id = s.id AND f.state = 'failed' \
               ORDER BY f.attempt DESC LIMIT 1) \
             WHERE w.root_run_id = ? AND s.state = 'blocked' \
             ORDER BY w.item_key IS NOT NULL, w.item_key, s.iteration, s.created_at, s.rowid",
        )
        .bind(root)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(BlockedRow::from).collect())
    }
}

impl From<RawBlocked> for BlockedRow {
    fn from(raw: RawBlocked) -> BlockedRow {
        let failure = match (raw.execution_id, raw.attempt) {
            (Some(execution), Some(attempt)) => Some(FailureRow {
                execution,
                attempt: number(attempt),
                code: raw.error_code.unwrap_or_default(),
                message: raw.error_message.unwrap_or_default(),
                resets_at: raw.resets_at,
            }),
            _ => None,
        };
        BlockedRow {
            key: raw.step_key,
            item: raw.item_key,
            iteration: raw.iteration.map(number),
            reason: block_reason(&raw.blocked_reason),
            failure,
        }
    }
}
