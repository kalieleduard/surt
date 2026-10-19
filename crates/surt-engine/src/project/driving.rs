use super::init::supported_git;
use super::{Error, Project, RunId};
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
        supported_git().await?;
        let actor = self.actor().await?;
        let store = self.store().await?;
        let driving = Driving {
            store: &store,
            layout: &self.layout,
            root: run,
            max_agents: usize::try_from(self.config.max_concurrent_agents.get())
                .unwrap_or(usize::MAX),
            worktrees: self
                .home
                .worktrees()
                .map(|worktrees| worktrees.join(&self.id)),
            actor: &actor,
            catalog: &self.catalog,
            stop_grace_period: self.stop_grace_period(),
        };
        let outcome = drive::drive(driving, runtime, progress, stop).await;
        store.close().await;
        outcome
    }
}
