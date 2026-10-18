mod impact;
mod locks;

use super::{Error, Layout, Project, Refusal, RunState};
use crate::cleanup::Left;
use crate::cleanup::Targets;
use crate::drive::graph;
use crate::store::{Store, SubjectRun};
pub use impact::{BranchFate, DeleteImpact, DeletedRun, Directory};
use locks::RunLocks;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Deletion {
    Deleted,
    Incomplete { left: Vec<Left> },
}

impl Project {
    pub async fn delete_impact(&self, subject: &str) -> Result<DeleteImpact, Error> {
        let store = self.store().await?;
        let impact = self.read_impact(&store, subject).await;
        store.close().await;
        impact
    }

    pub async fn delete(&self, subject: &str) -> Result<Deletion, Error> {
        let store = self.store().await?;
        let deletion = self.purge(&store, subject).await;
        store.close().await;
        deletion
    }

    async fn read_impact(&self, store: &Store, subject: &str) -> Result<DeleteImpact, Error> {
        let runs = deletable_runs(store, subject).await?;
        let targets = Targets::gather(store, &self.layout, subject, runs).await?;
        impact::read(store, &self.layout, subject, &targets).await
    }

    async fn purge(&self, store: &Store, subject: &str) -> Result<Deletion, Error> {
        let runs = deletable_runs(store, subject).await?;
        let locks = RunLocks::take(&self.layout, subject, &runs).await?;
        let targets = Targets::gather(store, &self.layout, subject, runs).await?;
        let left = targets.remove(&self.layout).await?;
        if !left.is_empty() {
            return Ok(Deletion::Incomplete { left });
        }
        #[cfg(feature = "test-support")]
        self.faults.before_purge().await;
        if !store.purge_subject(subject, &targets.runs).await? {
            return Err(Refusal::SubjectChangedWhileDeleting {
                subject: subject.to_owned(),
            }
            .into());
        }
        locks.release();
        Ok(Deletion::Deleted)
    }
}

async fn deletable_runs(store: &Store, subject: &str) -> Result<Vec<SubjectRun>, Error> {
    let runs = store.subject_runs(subject).await?;
    if runs.is_empty() {
        return Err(Refusal::UnknownSubject {
            reference: subject.to_owned(),
        }
        .into());
    }
    if runs
        .iter()
        .any(|run| run.run_state() == graph::RunState::Active)
    {
        return Err(Refusal::ActiveRunBlocksDelete {
            subject: subject.to_owned(),
        }
        .into());
    }
    Ok(runs)
}

fn relative(layout: &Layout, path: &std::path::Path) -> std::path::PathBuf {
    path.strip_prefix(layout.root())
        .unwrap_or(path)
        .to_path_buf()
}
