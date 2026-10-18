use std::collections::BTreeSet;
use std::path::Path;

use super::{Kept, Sweep};
use crate::drive::RunEnd;
use crate::git::{self, worktrees::WorktreeEntry};
use crate::platform::{self, ExclusiveLock};
use crate::project::Error;
use crate::reconcile::worktree::{self, Target};
use crate::store::TaskBranch;

const SALVAGE_REFS: &str = "refs/surt/salvage/";
const REFS: &str = "refs";
const WORKTREES: &str = "worktrees";

impl Sweep<'_> {
    pub(super) async fn repository(&mut self) -> Result<(), Error> {
        let branches = self.store.task_branches(self.root).await?;
        if branches.is_empty() {
            return Ok(());
        }
        let _git = self.lock_git().await?;
        let listed = match git::worktrees::list(self.layout.root()).await {
            Ok(listed) => listed,
            Err(error) => {
                self.leave(WORKTREES, error);
                return Ok(());
            }
        };
        let mut free = Vec::new();
        for branch in &branches {
            if self.remove_worktree(branch, &listed).await {
                free.push(branch);
            }
        }
        if self.end == RunEnd::Succeeded {
            self.delete_refs(&free, &branches).await?;
        }
        Ok(())
    }

    async fn lock_git(&self) -> Result<ExclusiveLock, Error> {
        let path = self.layout.git_lock();
        let waited = path.clone();
        tokio::task::spawn_blocking(move || platform::lock_exclusive(&waited))
            .await
            .map_err(Error::internal)?
            .map_err(|error| Error::io("lock", &path, error))
    }

    async fn remove_worktree(&mut self, branch: &TaskBranch, listed: &[WorktreeEntry]) -> bool {
        let Some(path) = branch.worktree_path.as_deref() else {
            return true;
        };
        if !listed
            .iter()
            .any(|entry| worktree::same_path(&entry.path, path))
        {
            if path.exists() {
                self.report.kept.push(Kept::ForeignPath {
                    path: path.to_path_buf(),
                });
            }
            return true;
        }
        if self.end == RunEnd::Cancelled
            && path.exists()
            && let Err(error) = self.salvage(branch, path).await
        {
            self.leave_path(path, error);
            return false;
        }
        match git::worktrees::remove(self.layout.root(), path).await {
            Ok(()) => {
                self.report.worktrees += 1;
                true
            }
            Err(error) => {
                self.leave_path(path, error);
                false
            }
        }
    }

    async fn salvage(
        &self,
        branch: &TaskBranch,
        path: &Path,
    ) -> Result<(), worktree::PrepareError> {
        let owner = branch.last_execution.as_deref().unwrap_or(&branch.run);
        let salvage_ref = format!("{SALVAGE_REFS}{owner}");
        let target = Target {
            repository: self.layout.root(),
            branch: &branch.ref_name,
            tip: &branch.tip_sha,
            path,
            owner: &branch.run,
            salvage_ref: &salvage_ref,
        };
        worktree::salvage_leftovers(&target).await.map(|_| ())
    }

    async fn delete_refs(
        &mut self,
        free: &[&TaskBranch],
        branches: &[TaskBranch],
    ) -> Result<(), Error> {
        let owners = self.owners(branches).await?;
        let repository = self.layout.root();
        let salvage_refs = match git::refs_under(repository, SALVAGE_REFS).await {
            Ok(listed) => owned(listed, &owners),
            Err(error) => {
                self.leave(REFS, error);
                return Ok(());
            }
        };
        let mut doomed = self.unmoved(free).await;
        let task_branches = doomed.len();
        let salvaged = salvage_refs.len();
        doomed.extend(salvage_refs);
        if doomed.is_empty() {
            return Ok(());
        }
        match git::delete_refs(repository, &doomed).await {
            Ok(()) => {
                self.report.task_branches += task_branches;
                self.report.salvage_refs += salvaged;
            }
            Err(error) => self.leave(REFS, error),
        }
        Ok(())
    }

    async fn unmoved(&mut self, free: &[&TaskBranch]) -> Vec<(String, String)> {
        let mut unmoved = Vec::new();
        for branch in free {
            match git::commit_sha(self.layout.root(), &branch.ref_name).await {
                Ok(Some(sha)) if sha == branch.tip_sha => {
                    unmoved.push((branch.ref_name.clone(), sha));
                }
                Ok(Some(_)) => self.report.kept.push(Kept::MovedBranch {
                    branch: git::short_branch(&branch.ref_name).to_owned(),
                }),
                Ok(None) => {}
                Err(error) => self.leave(&branch.ref_name, error),
            }
        }
        unmoved
    }

    async fn owners(&self, branches: &[TaskBranch]) -> Result<BTreeSet<String>, Error> {
        let mut owners: BTreeSet<String> = self
            .store
            .execution_ids(self.root)
            .await?
            .into_iter()
            .collect();
        owners.extend(branches.iter().map(|branch| branch.run.clone()));
        Ok(owners)
    }
}

fn owned(listed: Vec<(String, String)>, owners: &BTreeSet<String>) -> Vec<(String, String)> {
    listed
        .into_iter()
        .filter(|(name, _)| {
            name.strip_prefix(SALVAGE_REFS)
                .is_some_and(|owner| owners.contains(owner))
        })
        .collect()
}
