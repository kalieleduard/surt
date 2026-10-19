use super::Reconcile;
use crate::drive::{BaseMerge, Progress, Settled, settle_fast_forward};
use crate::git;
use crate::operations::Operation;
use crate::project::Error;
use crate::store::{Acceptance, PendingRefMove};

impl Reconcile<'_> {
    pub(super) async fn settle_fast_forward(
        &self,
        pending: PendingRefMove,
    ) -> Result<Option<Progress>, Error> {
        let settled = settle_fast_forward(
            self.store,
            self.repository,
            &pending.step_run,
            &pending.intended,
        )
        .await?;
        let Settled::Recorded(settled) = settled else {
            return Ok(None);
        };
        let started = self.attempt(&pending, Operation::MergeIntoBase).await?;
        let recorded = settled.data();
        let acceptance = Acceptance {
            execution: &started.id,
            outcome: settled.outcome(),
            outputs: &[],
            tip: None,
            integrated: None,
            recorded: Some(&recorded),
        };
        self.store.accept_execution(acceptance).await?;
        Ok(match settled {
            BaseMerge::Merged { to, .. } => Some(Progress::IntegrationRecorded {
                step: pending.step,
                branch: git::short_branch(&pending.intended.ref_name).to_owned(),
                merge: to,
            }),
            BaseMerge::Skipped(_) => None,
        })
    }
}
