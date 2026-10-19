use std::path::{Path, PathBuf};

use super::super::base_merge::{BaseMerge, NotMerged};
use super::super::driver::Driver;
use super::super::failure::{Failure, OperationError};
use super::super::operate::OperationLaunch;
use super::{INTERRUPTED, merged, reflog_message};
use crate::git::{self, FastForward, GitError};
use crate::runtime::AgentRuntime;
use crate::store::RefMove;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn fast_forward_in(
        &self,
        launch: &OperationLaunch,
        intended: &RefMove,
        worktree: &Path,
    ) -> Result<BaseMerge, OperationError> {
        let store = self.driving.store;
        store.intend_ref_move(&launch.row.id, intended).await?;
        let message = reflog_message(launch, intended);
        let moved = {
            let _git = self.git_lock().await?;
            git::merge_ff_only(worktree, &intended.new_sha, &message).await?
        };
        match moved {
            FastForward::Moved => Ok(merged(intended)),
            FastForward::WouldOverwrite => {
                store.forget_ref_move(&launch.row.id).await?;
                Ok(BaseMerge::Skipped(NotMerged::WorktreeNotClean))
            }
        }
    }
}

pub(super) async fn checked_out(
    repository: &Path,
    reference: &str,
) -> Result<Option<PathBuf>, GitError> {
    Ok(git::worktrees::list(repository)
        .await?
        .into_iter()
        .find(|entry| entry.branch.as_deref() == Some(reference))
        .map(|entry| entry.path))
}

pub(super) fn interrupted(intended: &RefMove) -> Failure {
    let worktree = intended.worktree_path.as_deref().unwrap_or(Path::new(""));
    Failure::not_retryable(
        INTERRUPTED,
        format!(
            "merge into {} was interrupted; {} may be partially updated",
            git::short_branch(&intended.ref_name),
            worktree.display()
        ),
    )
}
