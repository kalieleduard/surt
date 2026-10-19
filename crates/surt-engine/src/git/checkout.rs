use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{GitError, run, run_with};

const IN_PROGRESS_MARKERS: [&str; 5] = [
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "rebase-merge",
    "rebase-apply",
];
const OVERWRITE_REFUSALS: [&str; 2] = [
    "untracked working tree files would be overwritten",
    "local changes to the following files would be overwritten",
];
const REFLOG_ACTION: &str = "GIT_REFLOG_ACTION";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FastForward {
    Moved,
    WouldOverwrite,
}

pub(crate) async fn is_clean(worktree: &Path) -> Result<bool, GitError> {
    let changes = run(
        worktree,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--untracked-files=no",
        ],
    )
    .await?;
    Ok(changes.trim().is_empty() && !in_progress(worktree).await?)
}

async fn in_progress(worktree: &Path) -> Result<bool, GitError> {
    let mut args = vec!["rev-parse", "--path-format=absolute"];
    for marker in IN_PROGRESS_MARKERS {
        args.extend(["--git-path", marker]);
    }
    let paths = run(worktree, &args).await?;
    Ok(paths.lines().map(PathBuf::from).any(|path| path.exists()))
}

pub(crate) async fn merge_ff_only(
    worktree: &Path,
    tip: &str,
    reflog_action: &str,
) -> Result<FastForward, GitError> {
    let args = ["merge", "--ff-only", "--no-stat", "--quiet", tip];
    let env = [(REFLOG_ACTION, OsStr::new(reflog_action))];
    match run_with(worktree, &args, &env).await {
        Ok(_) => Ok(FastForward::Moved),
        Err(GitError::Failed { stderr, .. })
            if OVERWRITE_REFUSALS
                .iter()
                .any(|refusal| stderr.contains(refusal)) =>
        {
            Ok(FastForward::WouldOverwrite)
        }
        Err(error) => Err(error),
    }
}
