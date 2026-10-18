use std::time::Duration;

use serde_json::json;

use super::Reconcile;
use crate::drive::Progress;
use crate::platform;
use crate::project::Error;
use crate::store::{ExecutionEnd, StaleExecution};

const STOP_GRACE_PERIOD: Duration = Duration::from_secs(10);
const ORPHAN_STOPPED: &str = "execution.orphan_stopped";

impl Reconcile<'_> {
    pub(super) async fn stop_stale(&self) -> Result<Vec<Progress>, Error> {
        let mut found = Vec::new();
        for stale in self.store.stale_executions(self.root).await? {
            found.extend(self.end_stale(stale).await?);
        }
        Ok(found)
    }

    async fn end_stale(&self, stale: StaleExecution) -> Result<Option<Progress>, Error> {
        let stopped = match stale.identity.as_ref().filter(|_| stale.running) {
            Some(identity) if platform::stop(identity, STOP_GRACE_PERIOD).await => {
                Some(identity.pid)
            }
            _ => None,
        };
        if let Some(pid) = stopped {
            let data = json!({ "pid": pid });
            self.store
                .record_execution_event(&stale.id, ORPHAN_STOPPED, &data)
                .await?;
        }
        self.store
            .end_execution(&stale.id, &ExecutionEnd::Interrupted)
            .await?;
        let StaleExecution { step, attempt, .. } = stale;
        Ok(match stopped {
            Some(pid) => Some(Progress::OrphanStopped { step, attempt, pid }),
            None if stale.running => Some(Progress::Discarded { step, attempt }),
            None => None,
        })
    }
}
