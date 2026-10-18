use std::path::PathBuf;

use super::{Error, Project, RunId, open_store};
use crate::drive::StepLabel;
use crate::drive::graph::BlockReason;
use crate::store::{BlockedRow, FailureRow};

const LOG_FILES: [&str; 2] = ["stream.jsonl", "output.log"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockedStep {
    pub step: StepLabel,
    pub reason: BlockReason,
    pub last_failure: Option<LastFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastFailure {
    pub attempt: u32,
    pub code: String,
    pub message: String,
    pub resets_at: Option<i64>,
    pub log: Option<PathBuf>,
}

impl Project {
    pub async fn blocked_steps(&self, run: &RunId) -> Result<Vec<BlockedStep>, Error> {
        let store = open_store(&self.layout).await?;
        let rows = store.blocked_steps(run.as_str()).await;
        let subject = store.subject_of_run(run.as_str()).await;
        store.close().await;
        let subject = subject?;
        Ok(rows?
            .into_iter()
            .map(|row| self.blocked_step(&subject, row))
            .collect())
    }

    fn blocked_step(&self, subject: &str, row: BlockedRow) -> BlockedStep {
        BlockedStep {
            step: StepLabel {
                key: row.key,
                item: row.item,
                iteration: row.iteration,
            },
            reason: row.reason,
            last_failure: row
                .failure
                .map(|failure| self.last_failure(subject, failure)),
        }
    }

    fn last_failure(&self, subject: &str, failure: FailureRow) -> LastFailure {
        let logs = self.layout.execution_logs(subject, &failure.execution);
        let log = LOG_FILES
            .into_iter()
            .map(|name| logs.join(name))
            .find(|path| path.is_file())
            .and_then(|path| {
                path.strip_prefix(self.layout.root())
                    .ok()
                    .map(PathBuf::from)
            });
        LastFailure {
            attempt: failure.attempt,
            code: failure.code,
            message: failure.message,
            resets_at: failure.resets_at,
            log,
        }
    }
}
