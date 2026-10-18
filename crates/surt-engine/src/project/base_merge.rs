use super::status::recorded_base;
use super::{Commit, Error, Project, RunId, open_store};
use crate::drive::BaseMerge;
use crate::operations::Operation;
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
    let done = store
        .last_operation_done(run, Operation::MergeIntoBase.id())
        .await?;
    let Some(merge) = done.as_deref().and_then(BaseMerge::from_data) else {
        return Ok(None);
    };
    Ok(recorded_base(store, run)
        .await?
        .map(|base| BaseMergeReport { base, merge }))
}
