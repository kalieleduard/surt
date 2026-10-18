use super::{Error, Project, RunId, open_store};
use crate::drive::StepLabel;
use crate::drive::graph::BlockReason;
use crate::store::{BlockedRow, FailureRow};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockedStep {
    pub step: StepLabel,
    pub reason: BlockReason,
    pub last_failure: Option<LastFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastFailure {
    pub execution: String,
    pub attempt: u32,
    pub code: String,
    pub message: String,
    pub resets_at: Option<i64>,
}

impl Project {
    pub async fn blocked_steps(&self, run: &RunId) -> Result<Vec<BlockedStep>, Error> {
        let store = open_store(&self.layout).await?;
        let rows = store.blocked_steps(run.as_str()).await;
        store.close().await;
        Ok(rows?.into_iter().map(BlockedStep::from).collect())
    }
}

impl From<BlockedRow> for BlockedStep {
    fn from(row: BlockedRow) -> BlockedStep {
        BlockedStep {
            step: StepLabel {
                key: row.key,
                item: row.item,
                iteration: row.iteration,
            },
            reason: row.reason,
            last_failure: row.failure.map(LastFailure::from),
        }
    }
}

impl From<FailureRow> for LastFailure {
    fn from(failure: FailureRow) -> LastFailure {
        LastFailure {
            execution: failure.execution,
            attempt: failure.attempt,
            code: failure.code,
            message: failure.message,
            resets_at: failure.resets_at,
        }
    }
}
