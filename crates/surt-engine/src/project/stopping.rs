use super::{Error, Project, Refusal, RunId, open_store};
use crate::drive::cancel::wait_for_lock;
use crate::platform;
use crate::store::Store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopRequest {
    pub run: RunId,
    pub driver: u32,
    pub executions: usize,
}

impl Project {
    pub async fn request_stop(&self, subject: &str) -> Result<StopRequest, Error> {
        let actor = self.actor().await?;
        let store = open_store(&self.layout).await?;
        let requested = self.request(&store, subject, &actor).await;
        store.close().await;
        requested
    }

    pub async fn wait_for_driver(&self, run: &RunId) -> Result<(), Error> {
        let _lock = wait_for_lock(&self.layout.run_lock(run.as_str())).await?;
        Ok(())
    }

    async fn request(
        &self,
        store: &Store,
        subject: &str,
        actor: &str,
    ) -> Result<StopRequest, Error> {
        let runs = store.runs_to_show(Some(subject), true).await?;
        let Some(run) = runs.into_iter().next() else {
            let reference = subject.to_owned();
            return Err(Refusal::UnknownSubject { reference }.into());
        };
        let Some(driver) = run.driver().filter(platform::is_alive) else {
            let states = self.run_status(store, &run).await?.states;
            let subject = run.subject;
            return Err(Refusal::NoLiveDriver { subject, states }.into());
        };
        store.request_stop(&run.id, actor).await?;
        let executions = store.cancel_impact(&run.id).await?.executions;
        Ok(StopRequest {
            run: RunId(run.id),
            driver: driver.pid,
            executions: usize::try_from(executions).unwrap_or_default(),
        })
    }
}
