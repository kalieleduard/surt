use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use super::{PrepareError, Salvage, Target, io_error};
use crate::git::{self, GitError};

const LOCK_EXTENSION: &str = "lock";
const SALVAGE_INDEX: &str = "surt-salvage-index";
const INDEX_VARIABLE: &str = "GIT_INDEX_FILE";

const MARKERS: [(&str, &[&str]); 8] = [
    ("MERGE_HEAD", &["merge", "--quit"]),
    ("rebase-merge", &["rebase", "--quit"]),
    ("rebase-apply/applying", &["am", "--quit"]),
    ("rebase-apply", &["rebase", "--quit"]),
    ("CHERRY_PICK_HEAD", &["cherry-pick", "--quit"]),
    ("REVERT_HEAD", &["revert", "--quit"]),
    ("sequencer", &["cherry-pick", "--quit"]),
    ("BISECT_LOG", &["bisect", "reset", "HEAD"]),
];

struct State {
    head_branch: Option<String>,
    head: Option<String>,
    branch_tip: Option<String>,
    dirty: bool,
    in_progress: bool,
}

impl State {
    async fn read(target: &Target<'_>, admin: &Path) -> Result<State, GitError> {
        let worktree = target.path;
        Ok(State {
            head_branch: head_branch(worktree).await?,
            head: git::commit_sha(worktree, "HEAD").await?,
            branch_tip: git::commit_sha(worktree, target.branch).await?,
            dirty: !status(worktree).await?.is_empty(),
            in_progress: MARKERS
                .iter()
                .any(|(marker, _)| admin.join(marker).exists()),
        })
    }

    fn off_target(&self, target: &Target<'_>) -> bool {
        self.head_branch.as_deref() != Some(target.branch)
            || self.head.as_deref() != Some(target.tip)
            || self.branch_tip.as_deref() != Some(target.tip)
    }
}

pub(super) async fn reset(target: &Target<'_>) -> Result<Option<Salvage>, PrepareError> {
    let worktree = target.path;
    let admin = git::absolute_git_dir(worktree).await?;
    remove_own_locks(target, &admin).await?;
    let state = State::read(target, &admin).await?;
    let off_target = state.off_target(target);
    let salvage = if off_target || state.dirty || state.in_progress {
        save(target, &admin, &state).await?;
        Some(Salvage {
            reference: target.salvage_ref.to_owned(),
            off_target,
        })
    } else {
        None
    };
    for (marker, quit) in MARKERS {
        if admin.join(marker).exists() {
            git::run(worktree, quit).await?;
        }
    }
    let branch = target.short_branch();
    git::run(
        worktree,
        &["checkout", "--quiet", "--force", "-B", branch, target.tip],
    )
    .await?;
    git::run(worktree, &["clean", "-fd", "--quiet"]).await?;
    verify(target, &admin).await?;
    Ok(salvage)
}

async fn remove_own_locks(target: &Target<'_>, admin: &Path) -> Result<(), PrepareError> {
    let entries = fs::read_dir(admin).map_err(io_error("read", admin))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension() == Some(OsStr::new(LOCK_EXTENSION)) {
            remove_if_present(&path)?;
        }
    }
    let common = git::common_dir(target.repository).await?;
    remove_if_present(&common.join(format!("{}.{LOCK_EXTENSION}", target.branch)))
}

async fn save(target: &Target<'_>, admin: &Path, state: &State) -> Result<(), PrepareError> {
    let worktree = target.path;
    let index = admin.join(SALVAGE_INDEX);
    remove_if_present(&index)?;
    let env = [(INDEX_VARIABLE, index.as_os_str())];
    let read_tree: &[&str] = match state.head {
        Some(_) => &["read-tree", "HEAD"],
        None => &["read-tree", "--empty"],
    };
    git::run_with(worktree, read_tree, &env).await?;
    git::run_with(worktree, &["add", "-A"], &env).await?;
    let tree = git::run_with(worktree, &["write-tree"], &env).await?;
    let message = format!("surt salvage of {}", target.short_branch());
    let mut args = vec!["commit-tree", "--no-gpg-sign", tree.trim(), "-m", &message];
    let mut parents: Vec<&str> = Vec::new();
    for parent in [&state.head, &state.branch_tip].into_iter().flatten() {
        if !parents.contains(&parent.as_str()) {
            parents.push(parent);
        }
    }
    for parent in &parents {
        args.extend(["-p", parent]);
    }
    let commit = git::run(worktree, &args).await?;
    git::set_ref(worktree, target.salvage_ref, commit.trim()).await?;
    remove_if_present(&index)
}

async fn verify(target: &Target<'_>, admin: &Path) -> Result<(), PrepareError> {
    let state = State::read(target, admin).await?;
    let detail = if state.off_target(target) {
        format!("HEAD is not {} at {}", target.short_branch(), target.tip)
    } else if state.dirty {
        "changes remain".to_owned()
    } else if state.in_progress {
        "an operation is still in progress".to_owned()
    } else {
        return Ok(());
    };
    Err(PrepareError::Unverified {
        path: target.path.to_path_buf(),
        detail,
    })
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

fn remove_if_present(path: &Path) -> Result<(), PrepareError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => {
            Err(io_error("remove", path)(error))
        }
        _ => Ok(()),
    }
}
