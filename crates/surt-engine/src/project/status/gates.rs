use std::collections::BTreeSet;
use std::path::Path;

use super::{GateView, ItemListSummary, RevisionSummary};
use crate::artifacts::listed_items;
use crate::artifacts::working_copy::{self, Comparison};
use crate::project::gates::approval_step;
use crate::project::{Error, GateStatus, Layout, RevisionOrigin};
use crate::store::{GateViewRow, Producer, RootRunRow, Store, WorkingCopyArtifact};

const RUNNING: &str = "running";
const DONE: &str = "done";
const APPROVED: &str = "approved";

pub(super) async fn views(
    store: &Store,
    layout: &Layout,
    run: &RootRunRow,
) -> Result<Vec<GateView>, Error> {
    let root = run.id.as_str();
    let working_copy = store.working_copy(root).await?;
    let dir = layout.working_copy(&run.subject);
    let mut views = Vec::new();
    for row in store.gate_views(root).await? {
        let gate = approval_step(store, root, &row.definition, &row.key).await?;
        let artifact = gate.artifact;
        let latest = store.latest_revision_number(root, &artifact).await?;
        let status = status(&row, latest);
        let comparison = compare(&dir, working_copy.iter().find(|w| w.name == artifact))?;
        let revision = Revision {
            artifact: &artifact,
            status,
            item_list: gate.item_list,
        };
        views.push(GateView {
            revision: summary(store, root, revision).await?,
            step: row.key,
            artifact,
            status,
            decided_by: row.actor,
            decided_at: row.decided_at,
            working_copy: layout.relative(&dir),
            edited: comparison.edited,
            missing: comparison.missing,
        });
    }
    Ok(views)
}

pub(in crate::project) fn origin(producer: &Producer) -> RevisionOrigin {
    match producer {
        Producer::Step { .. } => RevisionOrigin::Agent,
        Producer::Human(_) => RevisionOrigin::Human,
    }
}

fn status(row: &GateViewRow, latest: Option<u32>) -> GateStatus {
    let decided = row.decided.and_then(|number| u32::try_from(number).ok());
    match (row.state.as_str(), decided) {
        (RUNNING, _) => latest.map_or(GateStatus::NotReached, |revision| GateStatus::Waiting {
            revision,
        }),
        (DONE, Some(revision)) if row.outcome.as_deref() == Some(APPROVED) => {
            GateStatus::Approved { revision }
        }
        (DONE, Some(revision)) => GateStatus::Rejected { revision },
        _ => GateStatus::NotReached,
    }
}

struct Revision<'a> {
    artifact: &'a str,
    status: GateStatus,
    item_list: bool,
}

async fn summary(
    store: &Store,
    root: &str,
    wanted: Revision<'_>,
) -> Result<Option<RevisionSummary>, Error> {
    let number = match wanted.status {
        GateStatus::Waiting { revision }
        | GateStatus::Approved { revision }
        | GateStatus::Rejected { revision } => revision,
        GateStatus::NotReached => return Ok(None),
    };
    let Some(id) = store
        .revision_id(root, wanted.artifact, Some(number))
        .await?
    else {
        return Ok(None);
    };
    let revision = store.revision(&id).await?;
    let item_list = wanted
        .item_list
        .then(|| item_list(&revision.files))
        .flatten();
    Ok(Some(RevisionSummary {
        number,
        origin: origin(&revision.producer),
        sha256: revision.sha256,
        files: revision.files.into_iter().map(|(path, _)| path).collect(),
        item_list,
    }))
}

fn item_list(files: &[(String, Vec<u8>)]) -> Option<ItemListSummary> {
    let items = listed_items(files)?;
    let item_files: BTreeSet<&String> = items.iter().flat_map(|item| item.files.values()).collect();
    Some(ItemListSummary {
        items: items.len(),
        item_files: item_files.len(),
    })
}

fn compare(dir: &Path, artifact: Option<&WorkingCopyArtifact>) -> Result<Comparison, Error> {
    let Some(artifact) = artifact.filter(|artifact| !artifact.pending) else {
        return Ok(Comparison {
            edited: Vec::new(),
            missing: Vec::new(),
        });
    };
    working_copy::compare(dir, &artifact.files).map_err(|error| Error::io("read", dir, error))
}
