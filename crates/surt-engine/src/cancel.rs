use std::path::Path;
use std::time::Duration;

use crate::cleanup;
use crate::drive::{Progress, ProgressSink, RunEnd};
use crate::platform::{self, ExclusiveLock};
use crate::project::{Error, Layout};
use crate::reconcile::Reconcile;
use crate::store::Store;

const LOCK_POLL: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cancellation {
    Cancelled,
    AlreadyCancelled,
    AlreadySucceeded,
}

pub(crate) struct Cancel<'a> {
    pub(crate) store: &'a Store,
    pub(crate) layout: &'a Layout,
    pub(crate) root: &'a str,
    pub(crate) actor: &'a str,
}

impl Cancel<'_> {
    pub(crate) async fn run(&self, progress: &ProgressSink) -> Result<Cancellation, Error> {
        let path = self.layout.run_lock(self.root);
        let (_lock, requested) = match try_lock(&path)? {
            Some(lock) => (lock, false),
            None => (self.request(&path, progress).await?, true),
        };
        let cancellation = match self.store.ending(self.root).await? {
            None => self.cancel_directly(progress).await?,
            Some(ending) => match ending.end {
                RunEnd::Cancelled if requested => Cancellation::Cancelled,
                RunEnd::Cancelled => Cancellation::AlreadyCancelled,
                RunEnd::Succeeded => Cancellation::AlreadySucceeded,
            },
        };
        if let Some(cleanup) = cleanup::clean_up(self.store, self.layout, self.root).await? {
            let _ = progress.send(Progress::CleanedUp(cleanup));
        }
        Ok(cancellation)
    }

    async fn request(&self, path: &Path, progress: &ProgressSink) -> Result<ExclusiveLock, Error> {
        self.store.request_cancel(self.root, self.actor).await?;
        let driver = self.store.driver(self.root).await?.pid;
        let driver = driver.and_then(|pid| u32::try_from(pid).ok());
        let _ = progress.send(Progress::CancelRequested { driver });
        loop {
            tokio::time::sleep(LOCK_POLL).await;
            if let Some(lock) = try_lock(path)? {
                return Ok(lock);
            }
        }
    }

    async fn cancel_directly(&self, progress: &ProgressSink) -> Result<Cancellation, Error> {
        let reconcile = Reconcile {
            store: self.store,
            repository: self.layout.root(),
            root: self.root,
        };
        for found in reconcile.settle().await? {
            let _ = progress.send(found);
        }
        self.store.cancel_run(self.root).await?;
        Ok(Cancellation::Cancelled)
    }
}

fn try_lock(path: &Path) -> Result<Option<ExclusiveLock>, Error> {
    platform::try_lock_exclusive(path).map_err(|error| Error::io("lock", path, error))
}
