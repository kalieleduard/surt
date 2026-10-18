use std::path::Path;

use super::Reconcile;
use crate::drive::Progress;
use crate::drive::graph::BlockReason;
use crate::git::{self, GitError};
use crate::operations::Operation;
use crate::project::Error;
use crate::runtime::FailureClass;
use crate::store::{
    Acceptance, ExecutionEnd, ExecutionKind, ExecutionStart, Integrated, PendingRefMove, RefMove,
    StartedExecution,
};

const MERGED: &str = "merged";
const MOVED: &str = "integration_branch_moved";
const NO_BRANCH: &str = "no branch";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Found {
    Moved,
    NotMoved,
    Elsewhere(Option<String>),
}

pub(crate) async fn find(repository: &Path, intended: &RefMove) -> Result<Found, GitError> {
    Ok(
        match git::commit_sha(repository, &intended.ref_name).await? {
            Some(sha) if sha == intended.new_sha => Found::Moved,
            Some(sha) if sha == intended.old_sha => Found::NotMoved,
            other => Found::Elsewhere(other),
        },
    )
}

impl Reconcile<'_> {
    pub(super) async fn settle_ref_moves(&self) -> Result<Vec<Progress>, Error> {
        let mut found = Vec::new();
        for pending in self.store.pending_ref_moves(self.root).await? {
            if pending.operation.as_deref().and_then(Operation::find)
                == Some(Operation::MergeIntoBase)
            {
                found.extend(self.settle_fast_forward(pending).await?);
                continue;
            }
            match find(self.repository, &pending.intended).await? {
                Found::Moved => found.push(self.confirm(pending).await?),
                Found::NotMoved => self.store.forget_ref_move(&pending.step_run).await?,
                Found::Elsewhere(sha) => found.push(self.refuse(pending, sha).await?),
            }
        }
        Ok(found)
    }

    async fn confirm(&self, pending: PendingRefMove) -> Result<Progress, Error> {
        let started = self.attempt(&pending, Operation::Integrate).await?;
        let merge = pending.intended.new_sha;
        let acceptance = Acceptance {
            execution: &started.id,
            outcome: MERGED,
            outputs: &[],
            tip: None,
            integrated: Some(Integrated {
                into: self.root,
                merge: &merge,
            }),
            recorded: None,
        };
        self.store.accept_execution(acceptance).await?;
        Ok(Progress::IntegrationRecorded {
            step: pending.step,
            branch: git::short_branch(&pending.intended.ref_name).to_owned(),
            merge,
        })
    }

    async fn refuse(
        &self,
        pending: PendingRefMove,
        found: Option<String>,
    ) -> Result<Progress, Error> {
        let started = self.attempt(&pending, Operation::Integrate).await?;
        let RefMove {
            ref_name, old_sha, ..
        } = &pending.intended;
        let message = format!(
            "{} moved outside surt (expected {old_sha}, found {})",
            git::short_branch(ref_name),
            found.as_deref().unwrap_or(NO_BRANCH)
        );
        let end = ExecutionEnd::Failed {
            class: FailureClass::NotRetryable,
            code: MOVED.to_owned(),
            message: message.clone(),
            resets_at: None,
        };
        let store = self.store;
        store.end_execution(&started.id, &end).await?;
        store
            .block_step(&pending.step_run, BlockReason::NotRetryable)
            .await?;
        store.forget_ref_move(&pending.step_run).await?;
        Ok(Progress::AttemptFailed {
            step: pending.step,
            attempt: started.attempt,
            code: MOVED.to_owned(),
            message,
        })
    }

    pub(super) async fn attempt(
        &self,
        pending: &PendingRefMove,
        operation: Operation,
    ) -> Result<StartedExecution, Error> {
        let start = ExecutionStart {
            step: &pending.step_run,
            kind: ExecutionKind::Operation { id: operation.id() },
            inputs: Vec::new(),
        };
        Ok(self.store.start_execution(start).await?)
    }
}
