use std::path::PathBuf;

use super::{Error, Layout, RunState, relative};
use crate::artifacts::working_copy::unrecorded_edits;
use crate::cleanup::{RefKind, Targets, size};
use crate::git;
use crate::store::{Store, SubjectRun};

const SUCCEEDED: &str = "succeeded";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteImpact {
    pub subject: String,
    pub runs: Vec<DeletedRun>,
    pub integration_branches: Vec<BranchFate>,
    pub task_branches: Vec<String>,
    pub salvage_refs: usize,
    pub directories: Vec<Directory>,
    pub unrecorded_edits: Vec<PathBuf>,
    pub revisions: usize,
    pub executions: usize,
    pub events: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeletedRun {
    pub run: u32,
    pub state: RunState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchFate {
    pub branch: String,
    pub run: u32,
    pub kept: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directory {
    pub path: PathBuf,
    pub bytes: u64,
}

pub(super) async fn read(
    store: &Store,
    layout: &Layout,
    subject: &str,
    targets: &Targets,
) -> Result<DeleteImpact, Error> {
    let counts = store.history_counts(subject).await?;
    Ok(DeleteImpact {
        subject: subject.to_owned(),
        runs: targets.runs.iter().map(deleted_run).collect(),
        integration_branches: integration_branches(targets),
        task_branches: names(targets, |kind| kind == RefKind::Task),
        salvage_refs: names(targets, |kind| kind == RefKind::Salvage).len(),
        directories: directories(layout, targets)?,
        unrecorded_edits: edits(store, layout, subject, targets.runs.last()).await?,
        revisions: count(counts.revisions),
        executions: count(counts.executions),
        events: count(counts.events),
    })
}

fn deleted_run(run: &SubjectRun) -> DeletedRun {
    DeletedRun {
        run: u32::try_from(run.ordinal).unwrap_or_default(),
        state: if run.state == SUCCEEDED {
            RunState::Succeeded
        } else {
            RunState::Cancelled
        },
    }
}

fn integration_branches(targets: &Targets) -> Vec<BranchFate> {
    let doomed = targets.refs.iter().filter_map(|doomed| match doomed.kind {
        RefKind::Integration { run } => Some(BranchFate {
            branch: git::short_branch(&doomed.name).to_owned(),
            run,
            kept: false,
        }),
        RefKind::Task | RefKind::Salvage => None,
    });
    let kept = targets.kept.iter().map(|kept| BranchFate {
        branch: git::short_branch(&kept.name).to_owned(),
        run: kept.run,
        kept: true,
    });
    let mut fates: Vec<BranchFate> = doomed.chain(kept).collect();
    fates.sort_by_key(|fate| fate.run);
    fates
}

fn names(targets: &Targets, wanted: impl Fn(RefKind) -> bool) -> Vec<String> {
    targets
        .refs
        .iter()
        .filter(|doomed| wanted(doomed.kind))
        .map(|doomed| git::short_branch(&doomed.name).to_owned())
        .collect()
}

fn directories(layout: &Layout, targets: &Targets) -> Result<Vec<Directory>, Error> {
    targets
        .directories
        .iter()
        .map(|path| {
            let bytes = size(path).map_err(|error| Error::io("measure", path, error))?;
            Ok(Directory {
                path: relative(layout, path),
                bytes,
            })
        })
        .collect()
}

async fn edits(
    store: &Store,
    layout: &Layout,
    subject: &str,
    last: Option<&SubjectRun>,
) -> Result<Vec<PathBuf>, Error> {
    let Some(last) = last else {
        return Ok(Vec::new());
    };
    let dir = layout.working_copy(subject);
    let mut edited = Vec::new();
    for artifact in store.working_copy(&last.id).await? {
        let paths = unrecorded_edits(&dir, &artifact.files)
            .map_err(|error| Error::io("read", &dir, error))?;
        edited.extend(paths.iter().map(|path| relative(layout, &dir.join(path))));
    }
    Ok(edited)
}

fn count(value: i64) -> usize {
    usize::try_from(value).unwrap_or_default()
}
