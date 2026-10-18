mod branch;
mod ref_moves;
mod stale;
pub(crate) mod worktree;

use std::path::Path;
use std::time::Duration;

use crate::drive::Progress;
use crate::project::Error;
use crate::store::Store;

pub(crate) use ref_moves::{Found, find};

pub(crate) struct Reconcile<'a> {
    pub(crate) store: &'a Store,
    pub(crate) repository: &'a Path,
    pub(crate) root: &'a str,
    /// How long stopping a dead Driver's surviving agent waits before a
    /// forced kill.
    pub(crate) stop_grace_period: Duration,
}

impl Reconcile<'_> {
    pub(crate) async fn settle(&self) -> Result<Vec<Progress>, Error> {
        let mut found = self.stop_stale().await?;
        found.extend(self.settle_ref_moves().await?);
        Ok(found)
    }
}
