use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::drive::graph::RunState;
use crate::git::{self, worktrees::WorktreeEntry};
use crate::project::{Error, Layout};
use crate::reconcile::worktree::same_path;
use crate::store::{Store, SubjectBranch, SubjectRun, number};

const SURT_BRANCHES: &str = "refs/heads/surt/";
const SALVAGE_REFS: &str = "refs/surt/salvage/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefKind {
    Integration { run: u32 },
    Item,
    Salvage,
}

pub(crate) struct DoomedRef {
    pub(crate) name: String,
    pub(crate) kind: RefKind,
}

pub(crate) struct KeptBranch {
    pub(crate) name: String,
    pub(crate) run: u32,
}

pub(crate) struct Worktree {
    pub(crate) path: PathBuf,
    pub(crate) locked: bool,
}

pub(crate) struct Targets {
    pub(crate) runs: Vec<SubjectRun>,
    pub(crate) worktrees: Vec<Worktree>,
    pub(crate) refs: Vec<DoomedRef>,
    pub(crate) kept: Vec<KeptBranch>,
    pub(crate) directories: Vec<PathBuf>,
}

impl Targets {
    pub(crate) async fn gather(
        store: &Store,
        layout: &Layout,
        subject: &str,
        runs: Vec<SubjectRun>,
    ) -> Result<Targets, Error> {
        let repository = layout.root();
        let branches = store.subject_branches(subject).await?;
        let listed = git::worktrees::list(repository).await?;
        let existing: BTreeSet<String> = git::refs_under(repository, SURT_BRANCHES)
            .await?
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        let (mut refs, kept) = branch_refs(&branches, &existing);
        refs.extend(salvage_refs(store, repository, subject).await?);
        let executions = store.subject_executions(subject).await?;
        Ok(Targets {
            runs,
            worktrees: worktrees(&branches, &listed),
            refs,
            kept,
            directories: directories(layout, subject, &executions),
        })
    }
}

fn worktrees(branches: &[SubjectBranch], listed: &[WorktreeEntry]) -> Vec<Worktree> {
    let recorded = branches
        .iter()
        .filter_map(|branch| branch.worktree_path.as_deref().map(Path::new));
    recorded
        .filter_map(|path| {
            let entry = listed.iter().find(|entry| same_path(&entry.path, path))?;
            Some(Worktree {
                path: path.to_path_buf(),
                locked: entry.lock_reason.is_some(),
            })
        })
        .collect()
}

fn branch_refs(
    branches: &[SubjectBranch],
    existing: &BTreeSet<String>,
) -> (Vec<DoomedRef>, Vec<KeptBranch>) {
    let mut doomed = Vec::new();
    let mut kept = Vec::new();
    for branch in branches.iter().filter(|b| existing.contains(&b.ref_name)) {
        let run = number(branch.ordinal);
        let name = branch.ref_name.clone();
        let kind = match (branch.item, branch.root_run_state() == RunState::Succeeded) {
            (true, _) => RefKind::Item,
            (false, true) => {
                kept.push(KeptBranch { name, run });
                continue;
            }
            (false, false) => RefKind::Integration { run },
        };
        doomed.push(DoomedRef { name, kind });
    }
    (doomed, kept)
}

async fn salvage_refs(
    store: &Store,
    repository: &Path,
    subject: &str,
) -> Result<Vec<DoomedRef>, Error> {
    let mut owners: BTreeSet<String> = store
        .subject_executions(subject)
        .await?
        .into_iter()
        .collect();
    owners.extend(store.subject_run_ids(subject).await?);
    let listed: BTreeMap<String, String> = git::refs_under(repository, SALVAGE_REFS)
        .await?
        .into_iter()
        .collect();
    Ok(listed
        .into_keys()
        .filter(|name| {
            name.strip_prefix(SALVAGE_REFS)
                .is_some_and(|owner| owners.contains(owner))
        })
        .map(|name| DoomedRef {
            name,
            kind: RefKind::Salvage,
        })
        .collect())
}

fn directories(layout: &Layout, subject: &str, executions: &[String]) -> Vec<PathBuf> {
    let mut directories = vec![layout.logs(subject), layout.working_copy(subject)];
    directories.extend(
        executions
            .iter()
            .map(|execution| layout.execution(execution)),
    );
    directories.retain(|path| path.symlink_metadata().is_ok());
    directories
}
