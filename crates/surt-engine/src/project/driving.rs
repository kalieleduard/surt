use super::{Error, Project, RunId, open_store};
use crate::drive::{self, DriveOutcome, Driving, ProgressSink};
use crate::runtime::{AgentRuntime, StopSignal};

impl Project {
    pub async fn drive(
        &self,
        run: &RunId,
        runtime: &impl AgentRuntime,
        progress: ProgressSink,
        stop: StopSignal,
    ) -> Result<DriveOutcome, Error> {
        self.refuse_agent_overrides()?;
        let actor = self.actor().await?;
        let store = open_store(&self.layout).await?;
        let driving = Driving {
            store: &store,
            layout: &self.layout,
            root: run.as_str(),
            max_agents: usize::try_from(self.config.max_concurrent_agents.get())
                .unwrap_or(usize::MAX),
            actor: &actor,
        };
        let outcome = drive::drive(driving, runtime, progress, stop).await;
        store.close().await;
        outcome
    }
}
