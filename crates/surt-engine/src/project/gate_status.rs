use super::gates::approval_step;
use super::{Error, Project, RunId, open_store};
use crate::store::{GateRow, Store};

const RUNNING: &str = "running";
const DONE: &str = "done";
const APPROVED: &str = "approved";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateStatus {
    NotReached,
    Waiting { revision: u32 },
    Approved { revision: u32 },
    Rejected { revision: u32 },
}

impl Project {
    pub async fn gate_status(&self, run: &RunId, step: &str) -> Result<GateStatus, Error> {
        let store = open_store(&self.layout).await?;
        let status = status(&store, run.as_str(), step).await;
        store.close().await;
        status
    }
}

async fn status(store: &Store, root: &str, key: &str) -> Result<GateStatus, Error> {
    let Some(gate) = store.latest_gate(root, key).await? else {
        return Ok(GateStatus::NotReached);
    };
    match (gate.state.as_str(), gate.decided()) {
        (RUNNING, _) => waiting(store, root, key, &gate).await,
        (DONE, Some(revision)) if gate.outcome.as_deref() == Some(APPROVED) => {
            Ok(GateStatus::Approved { revision })
        }
        (DONE, Some(revision)) => Ok(GateStatus::Rejected { revision }),
        _ => Ok(GateStatus::NotReached),
    }
}

async fn waiting(
    store: &Store,
    root: &str,
    key: &str,
    gate: &GateRow,
) -> Result<GateStatus, Error> {
    let reviewed = approval_step(store, root, &gate.definition, key).await?;
    let latest = store
        .latest_revision_number(root, &reviewed.artifact)
        .await?;
    Ok(
        latest.map_or(GateStatus::NotReached, |revision| GateStatus::Waiting {
            revision,
        }),
    )
}
