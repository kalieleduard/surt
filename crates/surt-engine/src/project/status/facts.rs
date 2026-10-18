use serde::Deserialize;

use super::{CleanupState, PendingIntegration, Unattended};
use crate::cleanup::{Kept, Left};
use crate::git;
use crate::platform;
use crate::project::{Commit, Error};
use crate::store::{RootRunRow, Store};
use crate::workflow::OnSuccess;

const BASE_ARTIFACT: &str = "base";
const ACTIVE: &str = "active";
const SUCCEEDED: &str = "succeeded";

#[derive(Default, Deserialize)]
struct CleanupData {
    #[serde(default)]
    kept: Vec<Kept>,
    #[serde(default)]
    left: Vec<Left>,
}

#[derive(Deserialize)]
struct BaseData {
    #[serde(rename = "ref")]
    reference: Option<String>,
    sha: String,
}

pub(in crate::project) async fn base(store: &Store, root: &str) -> Result<Option<Commit>, Error> {
    let Some(id) = store.revision_id(root, BASE_ARTIFACT, None).await? else {
        return Ok(None);
    };
    let revision = store.revision(&id).await?;
    Ok(revision.files.first().and_then(|(_, bytes)| {
        serde_json::from_slice::<BaseData>(bytes)
            .ok()
            .map(|base| Commit {
                sha: base.sha,
                branch: base.reference,
            })
    }))
}

pub(super) async fn unattended(
    store: &Store,
    root: &str,
    driven: bool,
) -> Result<Vec<Unattended>, Error> {
    if driven {
        return Ok(Vec::new());
    }
    Ok(store
        .stale_executions(root)
        .await?
        .into_iter()
        .filter(|execution| execution.running)
        .filter_map(|execution| {
            let identity = execution.identity.filter(platform::is_alive)?;
            Some(Unattended {
                step: execution.step,
                pid: identity.pid,
            })
        })
        .collect())
}

pub(super) async fn integrating(
    store: &Store,
    root: &str,
) -> Result<Vec<PendingIntegration>, Error> {
    Ok(store
        .pending_ref_moves(root)
        .await?
        .into_iter()
        .map(|pending| PendingIntegration {
            step: pending.step,
            branch: git::short_branch(&pending.intended.ref_name).to_owned(),
            from: pending.intended.old_sha,
            to: pending.intended.new_sha,
        })
        .collect())
}

pub(super) async fn cleanup(
    store: &Store,
    run: &RootRunRow,
    on_success: OnSuccess,
) -> Result<Option<CleanupState>, Error> {
    if run.state == ACTIVE {
        return Ok(None);
    }
    if run.state == SUCCEEDED && !run.cleaned && on_success == OnSuccess::Keep {
        return Ok(Some(CleanupState::Kept));
    }
    let data: CleanupData = store
        .last_cleanup(&run.id)
        .await?
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default();
    Ok(Some(if run.cleaned {
        CleanupState::Finished { kept: data.kept }
    } else {
        CleanupState::Pending {
            left: data.left,
            kept: data.kept,
        }
    }))
}
