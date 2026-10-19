use std::path::PathBuf;

use super::{Error, Project, RunId, open_store};
use crate::git;
use crate::store::PendingRefMove;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterruptedMerge {
    pub branch: String,
    pub worktree: PathBuf,
    pub from: String,
    pub to: String,
}

impl Project {
    pub async fn interrupted_merge(&self, run: &RunId) -> Result<Option<InterruptedMerge>, Error> {
        let store = open_store(&self.layout).await?;
        let pending = store.pending_ref_moves(run.as_str()).await;
        store.close().await;
        Ok(pending?.into_iter().find_map(interrupted))
    }
}

fn interrupted(pending: PendingRefMove) -> Option<InterruptedMerge> {
    if !pending.merges_into_base() {
        return None;
    }
    let intended = pending.intended;
    Some(InterruptedMerge {
        branch: git::short_branch(&intended.ref_name).to_owned(),
        worktree: intended.worktree_path?,
        from: intended.old_sha,
        to: intended.new_sha,
    })
}
