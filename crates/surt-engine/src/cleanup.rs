mod disk;
mod repository;

use std::fmt::Display;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use crate::drive::RunEnd;
use crate::project::{Error, Layout};
use crate::store::{CleanupRecord, Store};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    pub worktrees: usize,
    pub task_branches: usize,
    pub salvage_refs: usize,
    pub working_copy: bool,
    pub kept: Vec<Kept>,
    pub left: Vec<Left>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kept {
    UnrecordedEdit { path: PathBuf },
    MovedBranch { branch: String },
    ForeignPath { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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

pub(crate) async fn clean_up(
    store: &Store,
    layout: &Layout,
    root: &str,
) -> Result<Option<Cleanup>, Error> {
    let Some(ending) = store.ending(root).await? else {
        return Ok(None);
    };
    if ending.cleaned {
        return Ok(None);
    }
    let mut sweep = Sweep {
        store,
        layout,
        root,
        end: ending.end,
        report: Cleanup::default(),
    };
    sweep.working_copy().await?;
    sweep.executions().await?;
    sweep.repository().await?;
    let report = sweep.report;
    store.record_cleanup(root, record(&report)).await?;
    Ok(Some(report))
}

impl Sweep<'_> {
    fn leave(&mut self, item: impl Display, reason: impl Display) {
        self.report.left.push(Left {
            item: item.to_string(),
            reason: reason.to_string(),
        });
    }

    fn leave_path(&mut self, path: &Path, reason: impl Display) {
        let shown = self.relative(path);
        self.leave(shown.display(), reason);
    }

    fn relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(self.layout.root())
            .unwrap_or(path)
            .to_path_buf()
    }
}

fn record(report: &Cleanup) -> CleanupRecord {
    if report.is_complete() {
        return CleanupRecord::Finished(json!({ "kept": report.kept }));
    }
    CleanupRecord::Incomplete(json!({ "kept": report.kept, "left": report.left }))
}
