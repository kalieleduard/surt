use std::path::Path;

use crate::git::{self, GitError};

pub(super) const MARKERS: [(&str, &[&str]); 8] = [
    ("MERGE_HEAD", &["merge", "--quit"]),
    ("rebase-merge", &["rebase", "--quit"]),
    ("rebase-apply/applying", &["am", "--quit"]),
    ("rebase-apply", &["rebase", "--quit"]),
    ("CHERRY_PICK_HEAD", &["cherry-pick", "--quit"]),
    ("REVERT_HEAD", &["revert", "--quit"]),
    ("sequencer", &["cherry-pick", "--quit"]),
    ("BISECT_LOG", &["bisect", "reset", "HEAD"]),
];

pub(super) struct State {
    pub(super) head_branch: Option<String>,
    pub(super) head: Option<String>,
    pub(super) branch_tip: Option<String>,
    pub(super) dirty: bool,
    pub(super) in_progress: bool,
}

impl State {
    pub(super) async fn read(
        worktree: &Path,
        branch: &str,
        admin: &Path,
    ) -> Result<State, GitError> {
        Ok(State {
            head_branch: head_branch(worktree).await?,
            head: git::commit_sha(worktree, "HEAD").await?,
            branch_tip: git::commit_sha(worktree, branch).await?,
            dirty: !status(worktree).await?.is_empty(),
            in_progress: MARKERS
                .iter()
                .any(|(marker, _)| admin.join(marker).exists()),
        })
    }

    pub(super) fn at(&self, branch: &str, tip: &str) -> bool {
        self.head_branch.as_deref() == Some(branch)
            && self.head.as_deref() == Some(tip)
            && self.branch_tip.as_deref() == Some(tip)
    }
}

async fn head_branch(worktree: &Path) -> Result<Option<String>, GitError> {
    match git::run(worktree, &["symbolic-ref", "--quiet", "HEAD"]).await {
        Ok(branch) => Ok(Some(branch.trim().to_owned())),
        Err(GitError::Failed { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn status(worktree: &Path) -> Result<String, GitError> {
    git::run(
        worktree,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--untracked-files=all",
        ],
    )
    .await
}
