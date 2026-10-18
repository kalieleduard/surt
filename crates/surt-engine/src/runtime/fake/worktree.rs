use std::fs;
use std::path::Path;

use super::write_failure;
use crate::git::{self, GitError};
use crate::runtime::{FailureClass, RuntimeError};

const HEAD_TOKEN: &str = "$HEAD";

#[derive(Clone, Debug)]
pub(super) struct WorktreeCommit {
    pub(super) message: String,
    pub(super) files: Vec<(String, Vec<u8>)>,
}

impl WorktreeCommit {
    pub(super) async fn make(&self, worktree: &Path) -> Result<(), RuntimeError> {
        for (path, contents) in &self.files {
            let target = worktree.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(write_failure)?;
            }
            fs::write(&target, contents).map_err(write_failure)?;
        }
        git::run(worktree, &["add", "--all"])
            .await
            .map_err(git_failure)?;
        git::run(worktree, &["commit", "--quiet", "-m", &self.message])
            .await
            .map_err(git_failure)?;
        Ok(())
    }
}

pub(super) async fn resolve_head(
    contents: &[u8],
    worktree: &Path,
) -> Result<Vec<u8>, RuntimeError> {
    let Some(text) = std::str::from_utf8(contents)
        .ok()
        .filter(|text| text.contains(HEAD_TOKEN))
    else {
        return Ok(contents.to_vec());
    };
    let head = git::commit_sha(worktree, "HEAD")
        .await
        .map_err(git_failure)?
        .ok_or_else(|| {
            RuntimeError::new(
                FailureClass::Retryable,
                "fake_git",
                format!("{} has no HEAD", worktree.display()),
            )
        })?;
    Ok(text.replace(HEAD_TOKEN, &head).into_bytes())
}

fn git_failure(error: GitError) -> RuntimeError {
    RuntimeError::new(FailureClass::Retryable, "fake_git", error.to_string())
}
