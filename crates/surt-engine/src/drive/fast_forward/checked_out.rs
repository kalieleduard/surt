use std::path::{Path, PathBuf};

use super::super::base_merge::NotMerged;
use super::super::failure::Failure;
use super::INTERRUPTED;
use crate::git::{self, FastForward, GitError};
use crate::store::RefMove;

pub(super) async fn clean_checkout(
    repository: &Path,
    reference: &str,
) -> Result<Result<Option<PathBuf>, NotMerged>, GitError> {
    let Some(worktree) = checked_out(repository, reference).await? else {
        return Ok(Ok(None));
    };
    if git::is_clean(&worktree).await? {
        return Ok(Ok(Some(worktree)));
    }
    Ok(Err(NotMerged::WorktreeNotClean { worktree }))
}

pub(super) async fn move_base(
    repository: &Path,
    intended: &RefMove,
    message: &str,
) -> Result<FastForward, GitError> {
    let RefMove {
        ref_name,
        old_sha,
        new_sha,
        worktree_path,
    } = intended;
    match worktree_path {
        Some(worktree) => git::merge_ff_only(worktree, new_sha, message).await,
        None => git::fast_forward_ref(repository, ref_name, new_sha, old_sha, message)
            .await
            .map(|()| FastForward::Moved),
    }
}

pub(super) fn not_clean(intended: &RefMove) -> NotMerged {
    NotMerged::WorktreeNotClean {
        worktree: intended.worktree_path.clone().unwrap_or_default(),
    }
}

async fn checked_out(repository: &Path, reference: &str) -> Result<Option<PathBuf>, GitError> {
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
