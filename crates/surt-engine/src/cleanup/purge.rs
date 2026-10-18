mod disk;
mod targets;

use std::fmt::Display;
use std::path::Path;

use super::{Left, lock_git};
use crate::git;
use crate::project::{Error, Layout};

pub(crate) use disk::size;
pub(crate) use targets::{RefKind, Targets};

const REFS: &str = "refs";

impl Targets {
    pub(crate) async fn remove(&self, layout: &Layout) -> Result<Vec<Left>, Error> {
        let mut left = Vec::new();
        {
            let _git = lock_git(layout).await?;
            self.remove_worktrees(layout.root(), &mut left).await;
            self.delete_refs(layout.root(), &mut left).await;
        }
        for directory in &self.directories {
            if let Err(error) = disk::remove_dir(directory) {
                leave(&mut left, shown(layout, directory), error);
            }
        }
        Ok(left)
    }

    async fn remove_worktrees(&self, repository: &Path, left: &mut Vec<Left>) {
        for worktree in &self.worktrees {
            let removed = async {
                if worktree.locked {
                    git::worktrees::unlock(repository, &worktree.path).await?;
                }
                git::worktrees::remove(repository, &worktree.path).await
            };
            if let Err(error) = removed.await {
                leave(left, worktree.path.display(), error);
            }
        }
    }

    async fn delete_refs(&self, repository: &Path, left: &mut Vec<Left>) {
        if self.refs.is_empty() {
            return;
        }
        let names: Vec<String> = self.refs.iter().map(|doomed| doomed.name.clone()).collect();
        if let Err(error) = git::delete_refs_anywhere(repository, &names).await {
            leave(left, REFS, error);
        }
    }
}

fn shown<'a>(layout: &Layout, path: &'a Path) -> std::path::Display<'a> {
    path.strip_prefix(layout.root()).unwrap_or(path).display()
}

fn leave(left: &mut Vec<Left>, item: impl Display, reason: impl Display) {
    left.push(Left {
        item: item.to_string(),
        reason: reason.to_string(),
    });
}
