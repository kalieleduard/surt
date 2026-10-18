use std::path::{Path, PathBuf};

use super::{GitError, run};

const WORKTREE: &str = "worktree ";
const LOCKED: &str = "locked";
const BRANCH: &str = "branch ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeEntry {
    pub(crate) path: PathBuf,
    pub(crate) branch: Option<String>,
    pub(crate) lock_reason: Option<String>,
}

pub(crate) async fn list(repository: &Path) -> Result<Vec<WorktreeEntry>, GitError> {
    let output = run(repository, &["worktree", "list", "--porcelain", "-z"]).await?;
    Ok(parse(&output))
}

pub(crate) async fn add(
    repository: &Path,
    path: &Path,
    branch: &str,
    reason: &str,
) -> Result<(), GitError> {
    let path = path.to_string_lossy();
    run(
        repository,
        &[
            "worktree", "add", "--quiet", "--lock", "--reason", reason, &path, branch,
        ],
    )
    .await?;
    Ok(())
}

pub(crate) async fn remove(repository: &Path, path: &Path) -> Result<(), GitError> {
    let path = path.to_string_lossy();
    run(repository, &["worktree", "remove", "-f", "-f", &path]).await?;
    Ok(())
}

pub(crate) async fn unlock(repository: &Path, path: &Path) -> Result<(), GitError> {
    let path = path.to_string_lossy();
    run(repository, &["worktree", "unlock", &path]).await?;
    Ok(())
}

pub(crate) async fn repair(repository: &Path, path: &Path) -> Result<(), GitError> {
    let path = path.to_string_lossy();
    run(repository, &["worktree", "repair", &path]).await?;
    Ok(())
}

fn parse(output: &str) -> Vec<WorktreeEntry> {
    let mut entries = Vec::new();
    let mut fields = output.split('\0').peekable();
    while fields.peek().is_some() {
        let record: Vec<&str> = fields
            .by_ref()
            .take_while(|field| !field.is_empty())
            .collect();
        if let Some(entry) = entry(&record) {
            entries.push(entry);
        }
    }
    entries
}

fn entry(record: &[&str]) -> Option<WorktreeEntry> {
    let path = record.first()?.strip_prefix(WORKTREE)?;
    let lock_reason = record
        .iter()
        .find_map(|field| field.strip_prefix(LOCKED))
        .map(|reason| reason.trim_start().to_owned());
    let branch = record
        .iter()
        .find_map(|field| field.strip_prefix(BRANCH))
        .map(str::to_owned);
    Some(WorktreeEntry {
        path: PathBuf::from(path),
        branch,
        lock_reason,
    })
}
