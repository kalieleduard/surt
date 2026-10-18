mod evidence;
mod locate;
mod salvage;
mod state;

use std::io;
use std::path::{Path, PathBuf};

use crate::git::{self, GitError};

pub(crate) use evidence::{Claim, EvidenceError, prove};
pub(crate) use locate::{Located, Repair};

pub(crate) struct Target<'a> {
    pub(crate) repository: &'a Path,
    pub(crate) branch: &'a str,
    pub(crate) tip: &'a str,
    pub(crate) path: &'a Path,
    pub(crate) owner: &'a str,
    pub(crate) salvage_ref: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Prepared {
    pub(crate) located: Located,
    pub(crate) salvage: Option<Salvage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Salvage {
    pub(crate) reference: String,
    pub(crate) off_target: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PrepareError {
    #[error("{} exists but is not a worktree surt created", .0.display())]
    Foreign(PathBuf),
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("cannot {action} {}: {source}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("worktree {} is not clean after preparation: {detail}", path.display())]
    Unverified { path: PathBuf, detail: String },
}

impl Target<'_> {
    pub(crate) fn short_branch(&self) -> &str {
        git::short_branch(self.branch)
    }
}

pub(crate) async fn prepare(target: &Target<'_>) -> Result<Prepared, PrepareError> {
    if git::commit_sha(target.repository, target.branch)
        .await?
        .is_none()
    {
        git::create_ref(target.repository, target.branch, target.tip).await?;
    }
    let located = locate::locate(target).await?;
    let salvage = salvage::reset(target).await?;
    Ok(Prepared { located, salvage })
}

fn io_error(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> PrepareError {
    let path = path.to_path_buf();
    move |source| PrepareError::Io {
        action,
        path,
        source,
    }
}
