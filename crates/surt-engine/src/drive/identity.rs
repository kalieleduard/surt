use std::future::Future;

use tokio::sync::mpsc::UnboundedReceiver;

use super::driver::Driver;
use crate::platform;
use crate::project::Error;
use crate::runtime::AgentRuntime;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn tracking<T>(
        &self,
        execution: &str,
        work: impl Future<Output = T>,
        mut spawned: UnboundedReceiver<u32>,
    ) -> Result<T, Error> {
        tokio::pin!(work);
        loop {
            tokio::select! {
                done = &mut work => return Ok(done),
                Some(pid) = spawned.recv() => {
                    if let Some(identity) = platform::identify(pid) {
                        self.driving.store.record_process(execution, &identity).await?;
                    }
                }
            }
        }
    }
}
