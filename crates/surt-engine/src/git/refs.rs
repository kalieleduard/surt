use std::path::{Path, PathBuf};

use super::{GitError, run};

const NOT_AN_ANCESTOR: i32 = 1;

pub(crate) async fn is_valid_branch_name(cwd: &Path, name: &str) -> Result<bool, GitError> {
    match run(cwd, &["check-ref-format", "--branch", name]).await {
        Ok(_) => Ok(true),
        Err(GitError::Failed { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) async fn common_dir(cwd: &Path) -> Result<PathBuf, GitError> {
    let output = run(
        cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    Ok(PathBuf::from(output.trim()))
}

pub(crate) async fn absolute_git_dir(cwd: &Path) -> Result<PathBuf, GitError> {
    let output = run(cwd, &["rev-parse", "--absolute-git-dir"]).await?;
    Ok(PathBuf::from(output.trim()))
}

pub(crate) async fn is_ancestor(
    cwd: &Path,
    ancestor: &str,
    descendant: &str,
) -> Result<bool, GitError> {
    match run(cwd, &["merge-base", "--is-ancestor", ancestor, descendant]).await {
        Ok(_) => Ok(true),
        Err(GitError::Failed { status, .. }) if status.code() == Some(NOT_AN_ANCESTOR) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(crate) async fn create_ref(cwd: &Path, reference: &str, sha: &str) -> Result<(), GitError> {
    run(cwd, &["update-ref", "--no-deref", reference, sha, ""]).await?;
    Ok(())
}

pub(crate) async fn set_ref(cwd: &Path, reference: &str, sha: &str) -> Result<(), GitError> {
    run(cwd, &["update-ref", "--no-deref", reference, sha]).await?;
    Ok(())
}
