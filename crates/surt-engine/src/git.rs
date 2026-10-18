use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output, Stdio};
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

mod merge;
mod refs;
pub(crate) mod worktrees;

pub(crate) use merge::{CommitTree, MergeTree, commit_tree, merge_tree, move_ref};
pub(crate) use refs::{
    absolute_git_dir, common_dir, create_ref, delete_refs, is_ancestor, is_valid_branch_name,
    refs_under, set_ref,
};

pub(crate) const MIN_VERSION: GitVersion = GitVersion {
    major: 2,
    minor: 38,
    patch: 0,
};

const PORCELAIN_WORKTREE_PREFIX: &str = "worktree ";
const PORCELAIN_BARE: &str = "bare";
const PEEL_TO_COMMIT: &str = "^{commit}";
pub(crate) const BRANCH_NAMESPACE: &str = "refs/heads/";
const LOCK_FAILURE_PREFIX: &str = "Unable to create '";
const LOCK_FAILURE_SUFFIX: &str = "': File exists";
const LOCK_BACKOFF: [Duration; 6] = [
    Duration::from_millis(20),
    Duration::from_millis(40),
    Duration::from_millis(80),
    Duration::from_millis(160),
    Duration::from_millis(320),
    Duration::from_millis(640),
];
const PLATFORM_CONFIG: &[&str] = if cfg!(windows) {
    &["-c", "core.longpaths=true"]
} else {
    &[]
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GitVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl GitVersion {
    pub(crate) fn parse(output: &str) -> Option<GitVersion> {
        let number = output.trim().strip_prefix("git version ")?;
        let mut parts = number
            .split(|character: char| !character.is_ascii_digit())
            .map(str::parse::<u32>);
        let major = parts.next()?.ok()?;
        let minor = parts.next()?.ok()?;
        let patch = parts.next().and_then(Result::ok).unwrap_or_default();
        Some(GitVersion {
            major,
            minor,
            patch,
        })
    }
}

impl fmt::Display for GitVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GitError {
    #[error("git is not on PATH")]
    NotFound,
    #[error("cannot run git: {0}")]
    Spawn(#[source] io::Error),
    #[error("git {args} failed ({status}): {stderr}")]
    Failed {
        args: String,
        status: ExitStatus,
        stderr: String,
    },
    #[error("git lock held: {}", lock.display())]
    LockHeld { lock: PathBuf },
    #[error("unexpected output from git {args}: {output:?}")]
    Unexpected { args: String, output: String },
}

pub(crate) enum MainWorktree {
    Toplevel(PathBuf),
    Bare(PathBuf),
}

pub(crate) async fn version() -> Result<GitVersion, GitError> {
    let args = ["--version"];
    let output = run(Path::new("."), &args).await?;
    GitVersion::parse(&output).ok_or_else(|| unexpected(&args, output))
}

pub(crate) async fn committer_email(cwd: &Path) -> Result<Option<String>, GitError> {
    match run(cwd, &["var", "GIT_COMMITTER_IDENT"]).await {
        Ok(ident) => Ok(email_of(&ident)),
        Err(GitError::Failed { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) async fn main_worktree(cwd: &Path) -> Result<Option<MainWorktree>, GitError> {
    let args = ["worktree", "list", "--porcelain", "-z"];
    let output = match run(cwd, &args).await {
        Err(GitError::Failed { .. }) => return Ok(None),
        result => result?,
    };
    parse_main_worktree(&output)
        .map(Some)
        .ok_or_else(|| unexpected(&args, output))
}

pub(crate) async fn commit_sha(cwd: &Path, reference: &str) -> Result<Option<String>, GitError> {
    let peeled = format!("{reference}{PEEL_TO_COMMIT}");
    let args = [
        "rev-parse",
        "--verify",
        "--quiet",
        "--end-of-options",
        &peeled,
    ];
    match run(cwd, &args).await {
        Ok(sha) => Ok(Some(sha.trim().to_owned())),
        Err(GitError::Failed { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn short_branch(reference: &str) -> &str {
    reference
        .strip_prefix(BRANCH_NAMESPACE)
        .unwrap_or(reference)
}

pub(crate) async fn branch_name(cwd: &Path, reference: &str) -> Result<Option<String>, GitError> {
    let args = ["rev-parse", "--symbolic-full-name", reference];
    let full_name = run(cwd, &args).await?;
    Ok(full_name
        .trim()
        .strip_prefix(BRANCH_NAMESPACE)
        .map(str::to_owned))
}

fn parse_main_worktree(output: &str) -> Option<MainWorktree> {
    let mut fields = output.split('\0').take_while(|field| !field.is_empty());
    let path = PathBuf::from(fields.next()?.strip_prefix(PORCELAIN_WORKTREE_PREFIX)?);
    if fields.any(|field| field == PORCELAIN_BARE) {
        return Some(MainWorktree::Bare(path));
    }
    Some(MainWorktree::Toplevel(path))
}

fn email_of(ident: &str) -> Option<String> {
    let (_, after_open) = ident.split_once('<')?;
    let (email, _) = after_open.split_once('>')?;
    Some(email.to_owned())
}

fn unexpected(args: &[&str], output: String) -> GitError {
    GitError::Unexpected {
        args: args.join(" "),
        output,
    }
}

pub(crate) async fn run(cwd: &Path, args: &[&str]) -> Result<String, GitError> {
    run_with(cwd, args, &[]).await
}

pub(crate) async fn run_with(
    cwd: &Path,
    args: &[&str],
    env: &[(&str, &OsStr)],
) -> Result<String, GitError> {
    let mut delays = LOCK_BACKOFF.iter();
    loop {
        let result = invoke(cwd, args, env).await;
        let Some(lock) = held_lock(&result) else {
            return result;
        };
        let Some(delay) = delays.next() else {
            return Err(GitError::LockHeld { lock });
        };
        tracing::debug!(lock = %lock.display(), ?delay, "git lock held, backing off");
        tokio::time::sleep(*delay).await;
    }
}

fn held_lock(result: &Result<String, GitError>) -> Option<PathBuf> {
    let Err(GitError::Failed { stderr, .. }) = result else {
        return None;
    };
    let (_, after) = stderr.split_once(LOCK_FAILURE_PREFIX)?;
    let (lock, _) = after.split_once(LOCK_FAILURE_SUFFIX)?;
    Some(PathBuf::from(lock))
}

async fn invoke(cwd: &Path, args: &[&str], env: &[(&str, &OsStr)]) -> Result<String, GitError> {
    let output = output(cwd, args, env).await?;
    if !output.status.success() {
        return Err(failed(args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

async fn output(cwd: &Path, args: &[&str], env: &[(&str, &OsStr)]) -> Result<Output, GitError> {
    command(cwd, args)
        .envs(env.iter().copied())
        .output()
        .await
        .map_err(spawn_error)
}

async fn run_with_stdin(cwd: &Path, args: &[&str], input: &str) -> Result<String, GitError> {
    let mut child = command(cwd, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(GitError::Spawn)?;
    }
    let output = child.wait_with_output().await.map_err(GitError::Spawn)?;
    if !output.status.success() {
        return Err(failed(args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command(cwd: &Path, args: &[&str]) -> Command {
    tracing::debug!(cwd = %cwd.display(), ?args, "running git");
    let mut command = Command::new("git");
    command
        .args(PLATFORM_CONFIG)
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", "C")
        .kill_on_drop(true);
    command
}

fn spawn_error(error: io::Error) -> GitError {
    match error.kind() {
        io::ErrorKind::NotFound => GitError::NotFound,
        _ => GitError::Spawn(error),
    }
}

fn failed(args: &[&str], output: &Output) -> GitError {
    GitError::Failed {
        args: args.join(" "),
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    }
}

#[cfg(test)]
mod tests;
