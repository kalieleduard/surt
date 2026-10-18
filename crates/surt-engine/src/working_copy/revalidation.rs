use super::WorkingCopy;
use crate::project::{Error, Refusal};
use crate::store::Store;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Unapproved {
    pub(crate) artifact: String,
    pub(crate) gate: String,
    pub(crate) revision: u32,
}

impl WorkingCopy {
    pub(crate) async fn unapproved(&self, store: &Store) -> Result<Option<Unapproved>, Error> {
        let pending = store.unapproved_after_approval(&self.root).await?;
        Ok(self.gated.iter().find_map(|gated| {
            pending
                .iter()
                .find(|(artifact, _)| *artifact == gated.artifact)
                .map(|(artifact, revision)| Unapproved {
                    artifact: artifact.clone(),
                    gate: gated.gate.clone(),
                    revision: *revision,
                })
        }))
    }

    pub(crate) fn refuse(&self, unapproved: Unapproved) -> Refusal {
        Refusal::UnapprovedRevision {
            subject: self.subject.clone(),
            artifact: unapproved.artifact,
            revision: unapproved.revision,
        }
    }
}
