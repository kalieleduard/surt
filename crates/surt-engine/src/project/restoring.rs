use std::path::Path;

use super::{Error, Layout, Project, Refusal, RunId};
use crate::artifacts::working_copy::restoring::{self, FileChange};
use crate::artifacts::working_copy::{finish_pending_writes, unrecorded_edits};
use crate::platform;
use crate::store::{RestoreTarget, RootRunRow, Store, number};

const RESTORED: &str = "working_copy.restored";
const ACTIVE: &str = "active";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreImpact {
    pub artifact: String,
    pub revision: u32,
    pub changes: Vec<FileChange>,
}

struct Restoring<'a> {
    store: &'a Store,
    layout: &'a Layout,
    subject: &'a str,
    artifact: &'a str,
}

impl Project {
    pub async fn restore_impact(
        &self,
        subject: &str,
        artifact: &str,
    ) -> Result<RestoreImpact, Error> {
        let store = self.store().await?;
        let restoring = Restoring {
            store: &store,
            layout: &self.layout,
            subject,
            artifact,
        };
        let impact = restoring.impact().await;
        store.close().await;
        impact
    }

    pub async fn restore(&self, subject: &str, artifact: &str) -> Result<RestoreImpact, Error> {
        let store = self.store().await?;
        let restoring = Restoring {
            store: &store,
            layout: &self.layout,
            subject,
            artifact,
        };
        let restored = restoring.restore().await;
        store.close().await;
        restored
    }
}

impl Restoring<'_> {
    async fn impact(&self) -> Result<RestoreImpact, Error> {
        let root = self.active_root().await?;
        let _lock = self.lock(root.id.clone()).await?;
        Ok(self.settled(&root.id).await?.0)
    }

    async fn restore(&self) -> Result<RestoreImpact, Error> {
        let root = self.active_root().await?;
        let _lock = self.lock(root.id.clone()).await?;
        let (impact, target) = self.settled(&root.id).await?;
        if !impact.changes.is_empty() {
            let dir = self.layout.working_copy(self.subject);
            restoring::rewrite(self.store, &dir, &target.files, &impact.changes).await?;
            let details = restoring::details(self.artifact, impact.revision, &impact.changes);
            self.store
                .record_run_event(&root.id, RESTORED, &details)
                .await?;
        }
        Ok(impact)
    }

    async fn lock(&self, root: String) -> Result<platform::ExclusiveLock, Error> {
        match try_lock(&self.layout.run_lock(&root))? {
            Some(lock) => Ok(lock),
            None => Err(self.driver_live(root).await),
        }
    }

    async fn settled(&self, root: &str) -> Result<(RestoreImpact, RestoreTarget), Error> {
        let dir = self.layout.working_copy(self.subject);
        finish_pending_writes(self.store, &dir, root).await?;
        self.compare(root).await
    }

    async fn compare(&self, root: &str) -> Result<(RestoreImpact, RestoreTarget), Error> {
        let Some(target) = self.store.restore_target(root, self.artifact).await? else {
            return Err(Refusal::NotRestorable {
                subject: self.subject.to_owned(),
                artifact: self.artifact.to_owned(),
                restorable: self.store.restorable(root).await?,
            }
            .into());
        };
        let dir = self.layout.working_copy(self.subject);
        let impact = RestoreImpact {
            artifact: self.artifact.to_owned(),
            revision: target.revision,
            changes: restoring::impact(self.store, &dir, &target.files).await?,
        };
        Ok((impact, target))
    }

    async fn active_root(&self) -> Result<RootRunRow, Error> {
        let runs = self.store.runs_to_show(Some(self.subject), true).await?;
        let Some(root) = runs.into_iter().next() else {
            return Err(Refusal::UnknownSubject {
                reference: self.subject.to_owned(),
            }
            .into());
        };
        if root.state == ACTIVE {
            return Ok(root);
        }
        Err(self.terminal(&root).await)
    }

    async fn terminal(&self, root: &RootRunRow) -> Error {
        let files = match self.edited(&root.id).await {
            Ok(files) => files,
            Err(error) => return error,
        };
        Refusal::TerminalRun {
            subject: self.subject.to_owned(),
            run: number(root.ordinal),
            files,
        }
        .into()
    }

    async fn edited(&self, root: &str) -> Result<Vec<String>, Error> {
        let Some(target) = self.store.restore_target(root, self.artifact).await? else {
            return Ok(Vec::new());
        };
        let dir = self.layout.working_copy(self.subject);
        unrecorded_edits(&dir, &target.files).map_err(|error| Error::io("read", &dir, error))
    }

    async fn driver_live(&self, root: String) -> Error {
        match self.store.driver(&RunId(root)).await {
            Ok(driver) => Refusal::DriverLive {
                subject: driver.subject,
                pid: driver.pid.and_then(|pid| u32::try_from(pid).ok()),
                since: driver.since,
            }
            .into(),
            Err(error) => error.into(),
        }
    }
}

fn try_lock(path: &Path) -> Result<Option<platform::ExclusiveLock>, Error> {
    platform::try_lock_exclusive(path).map_err(|error| Error::io("lock", path, error))
}
