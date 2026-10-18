use super::{Error, Project, RunId, open_store};
use crate::cancel::{Cancel, Cancellation};
use crate::drive::ProgressSink;
use crate::git;
use crate::store::Store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancelImpact {
    pub executions: usize,
    pub tasks: usize,
    pub merged: usize,
    pub integration_branch: Option<String>,
}

impl Project {
    pub async fn cancel_impact(&self, run: &RunId) -> Result<CancelImpact, Error> {
        let store = open_store(&self.layout).await?;
        let impact = impact(&store, run.as_str()).await;
        store.close().await;
        impact
    }

    pub async fn cancel(&self, run: &RunId, progress: ProgressSink) -> Result<Cancellation, Error> {
        let actor = self.actor().await?;
        let store = open_store(&self.layout).await?;
        let cancel = Cancel {
            store: &store,
            layout: &self.layout,
            root: run.as_str(),
            actor: &actor,
        };
        let cancellation = cancel.run(&progress).await;
        store.close().await;
        cancellation
    }
}

async fn impact(store: &Store, root: &str) -> Result<CancelImpact, Error> {
    let row = store.cancel_impact(root).await?;
    let branch = store.run_branch(root).await?;
    Ok(CancelImpact {
        executions: count(row.executions),
        tasks: count(row.tasks),
        merged: count(row.merged),
        integration_branch: branch.map(|branch| git::short_branch(&branch.ref_name).to_owned()),
    })
}

fn count(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
