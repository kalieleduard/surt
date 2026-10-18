use std::future::Future;

use tokio::sync::mpsc::UnboundedReceiver;

use super::driver::Driver;
use crate::platform;
use crate::project::Error;
use crate::runtime::AgentRuntime;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn tracking<T, E>(
        &self,
        execution: &str,
        work: impl Future<Output = T>,
        mut events: UnboundedReceiver<E>,
        pid_of: fn(E) -> Option<u32>,
    ) -> Result<T, Error> {
        tokio::pin!(work);
        loop {
            tokio::select! {
                done = &mut work => return Ok(done),
                Some(event) = events.recv() => {
                    if let Some(identity) = pid_of(event).and_then(platform::identify) {
                        self.driving.store.record_process(execution, &identity).await?;
                    }
                }
            }
        }
    }
}
