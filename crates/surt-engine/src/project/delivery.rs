use super::{Error, Project, RunId, open_store};
use crate::git;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegrationBranch {
    pub name: String,
    pub tip: String,
}

impl Project {
    pub async fn integration_branch(
        &self,
        run: &RunId,
    ) -> Result<Option<IntegrationBranch>, Error> {
        let store = open_store(&self.layout).await?;
        let branch = store.run_branch(run.as_str()).await;
        store.close().await;
        Ok(branch?.map(|branch| IntegrationBranch {
            name: git::short_branch(&branch.ref_name).to_owned(),
            tip: branch.tip_sha,
        }))
    }
}
