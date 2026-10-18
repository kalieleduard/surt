use super::Reconcile;
use crate::git::{self, worktrees};
use crate::operations::Operation;
use crate::project::{Error, Refusal};

impl Reconcile<'_> {
    pub(crate) async fn check_integration_branch(&self) -> Result<Option<Refusal>, Error> {
        let store = self.store;
        let Some(recorded) = store.run_branch(self.root).await? else {
            return Ok(None);
        };
        let found = git::commit_sha(self.repository, &recorded.ref_name).await?;
        let branch = git::short_branch(&recorded.ref_name).to_owned();
        if found.as_deref() != Some(recorded.tip_sha.as_str()) {
            let created = Operation::CreateRunBranch.id();
            if found.is_none() && !store.operation_succeeded(self.root, created).await? {
                return Ok(None);
            }
            return Ok(Some(Refusal::IntegrationBranchMoved {
                subject: store.subject_of_run(self.root).await?,
                branch,
                expected: recorded.tip_sha,
                found,
            }));
        }
        let listed = worktrees::list(self.repository).await?;
        let Some(entry) = listed
            .into_iter()
            .find(|entry| entry.branch.as_deref() == Some(recorded.ref_name.as_str()))
        else {
            return Ok(None);
        };
        Ok(Some(Refusal::IntegrationBranchCheckedOut {
            subject: store.subject_of_run(self.root).await?,
            branch,
            path: entry.path,
        }))
    }
}
