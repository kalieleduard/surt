use std::path::Path;

use super::state::State;
use crate::git::{self, GitError};

pub(crate) struct Claim<'a> {
    pub(crate) worktree: &'a Path,
    pub(crate) branch: &'a str,
    pub(crate) start: &'a str,
    pub(crate) previous: &'a str,
    pub(crate) integration_tip: Option<&'a str>,
    pub(crate) sha: &'a str,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Unproven {
    #[error("{sha} is not a commit")]
    NotACommit { sha: String },
    #[error("{sha} is not checked out as the tip of {branch}")]
    NotCheckedOut { sha: String, branch: String },
    #[error("{sha} does not descend from the child run's start {start}")]
    NotFromStart { sha: String, start: String },
    #[error("{sha} does not descend from the conflict's integration tip {tip}")]
    NotFromIntegrationTip { sha: String, tip: String },
    #[error("{sha} is not a new commit")]
    NotNew { sha: String },
    #[error("the worktree has uncommitted changes")]
    Dirty,
    #[error("a merge, rebase, cherry-pick or revert is in progress")]
    InProgress,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EvidenceError {
    #[error(transparent)]
    Unproven(#[from] Unproven),
    #[error(transparent)]
    Git(#[from] GitError),
}

pub(crate) async fn prove(claim: &Claim<'_>) -> Result<(), EvidenceError> {
    let worktree = claim.worktree;
    let sha = claim.sha.to_owned();
    if git::commit_sha(worktree, claim.sha).await?.as_deref() != Some(claim.sha) {
        return Err(Unproven::NotACommit { sha }.into());
    }
    let admin = git::absolute_git_dir(worktree).await?;
    let state = State::read(worktree, claim.branch, &admin).await?;
    if !state.at(claim.branch, claim.sha) {
        let branch = git::short_branch(claim.branch).to_owned();
        return Err(Unproven::NotCheckedOut { sha, branch }.into());
    }
    if !git::is_ancestor(worktree, claim.start, claim.sha).await? {
        let start = claim.start.to_owned();
        return Err(Unproven::NotFromStart { sha, start }.into());
    }
    if let Some(tip) = claim.integration_tip
        && !git::is_ancestor(worktree, tip, claim.sha).await?
    {
        let tip = tip.to_owned();
        return Err(Unproven::NotFromIntegrationTip { sha, tip }.into());
    }
    if claim.sha == claim.previous {
        return Err(Unproven::NotNew { sha }.into());
    }
    if state.dirty {
        return Err(Unproven::Dirty.into());
    }
    if state.in_progress {
        return Err(Unproven::InProgress.into());
    }
    Ok(())
}
