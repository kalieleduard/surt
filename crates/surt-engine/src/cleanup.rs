mod disk;
mod purge;
mod repository;

pub(crate) use purge::{RefKind, Targets, size};
pub(crate) use repository::lock_git;

use std::fmt::Display;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::drive::graph::RunId;
use crate::drive::{Frozen, Progress, ProgressSink, RunEnd};
use crate::project::{Error, Layout};
use crate::store::{CleanupRecord, Ending, Store};
use crate::workflow::OnSuccess;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    pub worktrees: usize,
    pub child_branches: usize,
    pub salvage_refs: usize,
    pub working_copy: bool,
    pub kept: Vec<Kept>,
    pub left: Vec<Left>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kept {
    UnrecordedEdit { path: PathBuf },
    MovedBranch { branch: String },
    ForeignPath { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Left {
    pub item: String,
    pub reason: String,
}

impl Cleanup {
    pub fn is_complete(&self) -> bool {
        self.left.is_empty()
    }
}

struct Sweep<'a> {
    store: &'a Store,
    layout: &'a Layout,
    root: &'a str,
    end: RunEnd,
    report: Cleanup,
}

pub(crate) async fn finish(
    store: &Store,
    layout: &Layout,
    run: &RunId,
    progress: &ProgressSink,
) -> Result<(), Error> {
    let Some(ending) = store.ending(run).await? else {
        return Ok(());
    };
    if ending.cleaned {
        return Ok(());
    }
    if kept(store, run, ending).await? {
        let _ = progress.send(Progress::RunKept);
        return Ok(());
    }
    let report = clean_up(store, layout, run, ending.end).await?;
    let _ = progress.send(Progress::CleanedUp(report));
    Ok(())
}

pub(crate) async fn is_kept(store: &Store, run: &RunId) -> Result<bool, Error> {
    match store.ending(run).await? {
        Some(ending) => kept(store, run, ending).await,
        None => Ok(false),
    }
}

pub(crate) async fn clear_kept_working_copy(
    store: &Store,
    layout: &Layout,
    run: &RunId,
) -> Result<(), Error> {
    let Some(ending) = store.ending(run).await? else {
        return Ok(());
    };
    if !kept(store, run, ending).await? {
        return Ok(());
    }
    let mut sweep = Sweep::new(store, layout, run, ending.end);
    sweep.working_copy().await?;
    match sweep.report.left.into_iter().next() {
        None => Ok(()),
        Some(left) => Err(Error::internal(io::Error::other(format!(
            "{}: {}",
            left.item, left.reason
        )))),
    }
}

async fn kept(store: &Store, run: &RunId, ending: Ending) -> Result<bool, Error> {
    if ending.cleaned || ending.end != RunEnd::Succeeded {
        return Ok(false);
    }
    let frozen = Frozen::from_files(store.frozen_files(run.as_str()).await?)?;
    Ok(frozen.on_success == OnSuccess::Keep)
}

async fn clean_up(
    store: &Store,
    layout: &Layout,
    run: &RunId,
    end: RunEnd,
) -> Result<Cleanup, Error> {
    let mut sweep = Sweep::new(store, layout, run, end);
    sweep.working_copy().await?;
    sweep.executions().await?;
    sweep.repository().await?;
    let report = sweep.report;
    store.record_cleanup(run.as_str(), record(&report)).await?;
    Ok(report)
}

impl<'a> Sweep<'a> {
    fn new(store: &'a Store, layout: &'a Layout, run: &'a RunId, end: RunEnd) -> Sweep<'a> {
        Sweep {
            store,
            layout,
            root: run.as_str(),
            end,
            report: Cleanup::default(),
        }
    }

    fn leave(&mut self, item: impl Display, reason: impl Display) {
        self.report.left.push(Left {
            item: item.to_string(),
            reason: reason.to_string(),
        });
    }

    fn leave_path(&mut self, path: &Path, reason: impl Display) {
        let shown = self.layout.relative(path);
        self.leave(shown.display(), reason);
    }
}

fn record(report: &Cleanup) -> CleanupRecord {
    if report.is_complete() {
        return CleanupRecord::Finished(json!({ "kept": report.kept }));
    }
    CleanupRecord::Incomplete(json!({ "kept": report.kept, "left": report.left }))
}
