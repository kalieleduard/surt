use super::status::{recorded_base, recorded_merge};
use super::{Commit, Error, Project, RunId, open_store};
use crate::drive::BaseMerge;
use crate::store::Store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseMergeReport {
    pub base: Commit,
    pub merge: BaseMerge,
}

impl Project {
    pub async fn base_merge(&self, run: &RunId) -> Result<Option<BaseMergeReport>, Error> {
        let store = open_store(&self.layout).await?;
        let report = recorded(&store, run.as_str()).await;
        store.close().await;
        report
    }
}

async fn recorded(store: &Store, run: &str) -> Result<Option<BaseMergeReport>, Error> {
    let Some(merge) = recorded_merge(store, run).await? else {
        return Ok(None);
    };
    Ok(recorded_base(store, run)
        .await?
        .map(|base| BaseMergeReport { base, merge }))
}
