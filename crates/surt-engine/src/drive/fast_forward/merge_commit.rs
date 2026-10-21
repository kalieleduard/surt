use std::path::Path;

use super::super::base_merge::NotMerged;
use super::super::branches::run_stem;
use super::super::driver::Driver;
use super::super::failure::OperationError;
use super::super::operate::OperationLaunch;
use crate::git::{self, CommitTree, GitError, MergeTree};
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::RefMove;

pub(super) struct Planned {
    pub(super) intended: RefMove,
    pub(super) merge_commit: bool,
}

impl Planned {
    pub(super) fn fast_forward(intended: RefMove) -> Planned {
        Planned {
            intended,
            merge_commit: false,
        }
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    /// The base moved since the run started: turn `intended` (base tip to run
    /// tip) into a fast-forward when the run already contains the base, into a
    /// merge commit on top of the base when they combine cleanly, or into the
    /// reason the base stays as it is.
    pub(super) async fn combine_with_moved_base(
        &self,
        launch: &OperationLaunch,
        intended: RefMove,
    ) -> Result<Result<Planned, NotMerged>, OperationError> {
        let root = self.driving.layout.root();
        let tree = {
            let _git = self.git_lock().await?;
            if git::is_ancestor(root, &intended.old_sha, &intended.new_sha).await? {
                return Ok(Ok(Planned::fast_forward(intended)));
            }
            git::merge_tree(root, &intended.old_sha, &intended.new_sha).await?
        };
        let tree = match tree {
            MergeTree::Clean { tree } => tree,
            MergeTree::Conflicted { paths } => return Ok(Err(NotMerged::Conflict { paths })),
        };
        let merge = self.commit_base_merge(launch, &intended, &tree).await?;
        Ok(Ok(Planned {
            intended: RefMove {
                new_sha: merge,
                ..intended
            },
            merge_commit: true,
        }))
    }

    async fn commit_base_merge(
        &self,
        launch: &OperationLaunch,
        intended: &RefMove,
        tree: &str,
    ) -> Result<String, OperationError> {
        let started_at = self
            .driving
            .store
            .step_started_at(&launch.row.id)
            .await?
            .ok_or_else(|| Error::internal(NotStarted))?;
        let stem = run_stem(&launch.subject, launch.ordinal);
        let message = format!(
            "Merge {stem} into {}\n\nSurt-Run: {stem}",
            git::short_branch(&intended.ref_name)
        );
        let commit = CommitTree {
            tree,
            parents: [&intended.old_sha, &intended.new_sha],
            message: &message,
            unix_time: started_at,
        };
        let _git = self.git_lock().await?;
        Ok(git::commit_tree(self.driving.layout.root(), &commit).await?)
    }
}

/// A move made by surt is a merge commit when it merges the base tip it
/// started from with a run tip that does not contain that base tip; a
/// fast-forward to a run tip never is, even if that tip is itself a merge.
pub(super) async fn is_merge_commit(
    repository: &Path,
    intended: &RefMove,
) -> Result<bool, GitError> {
    let parents = git::commit_parents(repository, &intended.new_sha).await?;
    let [first, second] = parents.as_slice() else {
        return Ok(false);
    };
    Ok(*first == intended.old_sha && !git::is_ancestor(repository, first, second).await?)
}

#[derive(Debug, thiserror::Error)]
#[error("the merge step has no start time")]
struct NotStarted;
