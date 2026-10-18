use std::path::{Path, PathBuf};

use super::error::{Error, Refusal};
use super::layout::Layout;
use crate::git::{self, MainWorktree};

const DOT_SURT: &str = ".surt";
const GLOBAL_CONFIG: &str = "config.toml";

/// The machine's `~/.surt/` (`%USERPROFILE%\.surt\` on Windows), when there
/// is a home directory: the managed worktrees and the global configuration.
#[derive(Clone)]
pub(crate) struct SurtHome {
    worktrees: Option<PathBuf>,
    config: Option<PathBuf>,
}

impl SurtHome {
    pub(crate) fn from_env() -> SurtHome {
        match std::env::home_dir() {
            Some(home) => SurtHome::at(&home),
            None => SurtHome {
                worktrees: None,
                config: None,
            },
        }
    }

    pub(crate) fn at(home: &Path) -> SurtHome {
        SurtHome {
            worktrees: Some(worktrees_under(home)),
            config: Some(home.join(DOT_SURT).join(GLOBAL_CONFIG)),
        }
    }

    pub(crate) fn worktrees(&self) -> Option<&Path> {
        self.worktrees.as_deref()
    }

    pub(crate) fn config(&self) -> Option<&Path> {
        self.config.as_deref()
    }

    fn manages(&self, path: &Path) -> bool {
        self.worktrees
            .as_deref()
            .is_some_and(|worktrees| path.starts_with(worktrees))
    }
}

fn worktrees_under(home: &Path) -> PathBuf {
    dunce::canonicalize(home)
        .unwrap_or_else(|_| home.to_path_buf())
        .join(DOT_SURT)
        .join("worktrees")
}

pub(crate) async fn outside_managed_worktrees(
    cwd: &Path,
    home: &SurtHome,
) -> Result<PathBuf, Error> {
    let cwd = canonical(cwd)?;
    if home.manages(&cwd) {
        let project = main_toplevel(&cwd).await.ok().flatten();
        return Err(Refusal::InsideManagedWorktree { path: cwd, project }.into());
    }
    Ok(cwd)
}

pub(crate) async fn main_toplevel(cwd: &Path) -> Result<Option<PathBuf>, Error> {
    match git::main_worktree(cwd).await? {
        None => Ok(None),
        Some(MainWorktree::Bare(path)) => Err(Refusal::BareRepository { path }.into()),
        Some(MainWorktree::Toplevel(path)) => canonical(&path).map(Some),
    }
}

pub(crate) async fn find_project(cwd: &Path, home: &SurtHome) -> Result<Layout, Error> {
    let cwd = outside_managed_worktrees(cwd, home).await?;
    let Some(toplevel) = main_toplevel(&cwd).await? else {
        return Err(Refusal::NotAProject { root: cwd }.into());
    };
    let layout = Layout::new(toplevel);
    if !layout.database().is_file() {
        return Err(Refusal::NotAProject {
            root: layout.root().to_path_buf(),
        }
        .into());
    }
    Ok(layout)
}

fn canonical(path: &Path) -> Result<PathBuf, Error> {
    dunce::canonicalize(path).map_err(|error| Error::io("resolve", path, error))
}
