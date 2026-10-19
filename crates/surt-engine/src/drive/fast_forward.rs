use std::path::Path;

use super::base_merge::{BaseMerge, NotMerged};
use super::branches::run_stem;
use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::inputs::text;
use super::operate::{Finished, OperationLaunch};
use crate::git::{self, BRANCH_NAMESPACE, FastForward, GitError};
use crate::project::{Error, MergeIntoBase};
use crate::reconcile::{Found, find};
use crate::runtime::AgentRuntime;
use crate::store::{RefMove, Store};
use checked_out::{clean_checkout, interrupted, move_base, not_clean};

mod checked_out;

const BASE_INPUT: &str = "base";
const INTEGRATION_INPUT: &str = "integration";
const REF_FIELD: &str = "ref";
const TIP_FIELD: &str = "tip";
const INTERRUPTED: &str = "merge_into_base_interrupted";

pub(crate) enum Settled {
    Recorded(BaseMerge),
    Redo,
    Interrupted,
}

pub(crate) async fn settle_fast_forward(
    store: &Store,
    repository: &Path,
    step: &str,
    intended: &RefMove,
) -> Result<Settled, Error> {
    match find(repository, intended).await? {
        Found::Moved => Ok(Settled::Recorded(merged(intended))),
        Found::NotMoved => {
            if let Some(worktree) = &intended.worktree_path
                && worktree.is_dir()
                && !git::is_clean(worktree).await?
            {
                return Ok(Settled::Interrupted);
            }
            store.forget_ref_move(step).await?;
            Ok(Settled::Redo)
        }
        Found::Elsewhere(found) => {
            store.forget_ref_move(step).await?;
            Ok(Settled::Recorded(base_moved(found)))
        }
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn merge_into_base(
        &self,
        launch: &OperationLaunch,
    ) -> Result<Finished, OperationError> {
        let (store, root) = (self.driving.store, self.driving.layout.root());
        if let Some(intended) = store.ref_move(&launch.row.id).await? {
            match settle_fast_forward(store, root, &launch.row.id, &intended).await? {
                Settled::Recorded(settled) => return Ok(recorded(settled)),
                Settled::Interrupted => return Err(interrupted(&intended).into()),
                Settled::Redo => {}
            }
        }
        Ok(recorded(self.decide_fast_forward(launch).await?))
    }

    async fn decide_fast_forward(
        &self,
        launch: &OperationLaunch,
    ) -> Result<BaseMerge, OperationError> {
        if self.frozen.merge_into_base == MergeIntoBase::Never {
            return Ok(BaseMerge::Skipped(NotMerged::Disabled));
        }
        let intended = match self.intended_move(launch).await? {
            Ok(intended) => intended,
            Err(skipped) => return Ok(BaseMerge::Skipped(skipped)),
        };
        if intended.old_sha == intended.new_sha {
            return Ok(merged(&intended));
        }
        self.fast_forward(launch, &intended).await
    }

    async fn intended_move(
        &self,
        launch: &OperationLaunch,
    ) -> Result<Result<RefMove, NotMerged>, OperationError> {
        let base = self.input_json(&launch.started, BASE_INPUT).await?;
        let Some(branch) = base.as_ref().and_then(|base| text(base, REF_FIELD)) else {
            return Ok(Err(NotMerged::NoBaseBranch));
        };
        let reference = format!("{BRANCH_NAMESPACE}{branch}");
        let base_sha = self.input_sha(launch, BASE_INPUT).await?;
        let root = self.driving.layout.root();
        let found = git::commit_sha(root, &reference).await?;
        if found.as_deref() != Some(base_sha.as_str()) {
            return Ok(Err(NotMerged::BaseMoved { found }));
        }
        let worktree_path = match clean_checkout(root, &reference).await? {
            Ok(worktree_path) => worktree_path,
            Err(skipped) => return Ok(Err(skipped)),
        };
        Ok(Ok(RefMove {
            ref_name: reference,
            old_sha: base_sha,
            new_sha: self.delivered_tip(launch).await?,
            worktree_path,
        }))
    }

    async fn delivered_tip(&self, launch: &OperationLaunch) -> Result<String, OperationError> {
        self.input_text(&launch.started, INTEGRATION_INPUT, TIP_FIELD)
            .await?
            .ok_or_else(|| {
                Failure::invalid_input("the integration input names no tip".into()).into()
            })
    }

    async fn fast_forward(
        &self,
        launch: &OperationLaunch,
        intended: &RefMove,
    ) -> Result<BaseMerge, OperationError> {
        let store = self.driving.store;
        store.intend_ref_move(&launch.row.id, intended).await?;
        let message = reflog_message(launch, intended);
        let root = self.driving.layout.root();
        let moved = {
            let _git = self.git_lock().await?;
            move_base(root, intended, &message).await
        };
        match moved {
            Ok(FastForward::Moved) => Ok(merged(intended)),
            Ok(FastForward::WouldOverwrite) => {
                store.forget_ref_move(&launch.row.id).await?;
                Ok(BaseMerge::Skipped(not_clean(intended)))
            }
            Err(failed @ GitError::Failed { .. }) if intended.worktree_path.is_none() => {
                self.settle_failed_swap(launch, intended, failed).await
            }
            Err(error) => Err(error.into()),
        }
    }

    async fn settle_failed_swap(
        &self,
        launch: &OperationLaunch,
        intended: &RefMove,
        failed: GitError,
    ) -> Result<BaseMerge, OperationError> {
        let (store, root) = (self.driving.store, self.driving.layout.root());
        match settle_fast_forward(store, root, &launch.row.id, intended).await? {
            Settled::Recorded(settled) => Ok(settled),
            Settled::Redo | Settled::Interrupted => Err(failed.into()),
        }
    }
}

fn reflog_message(launch: &OperationLaunch, intended: &RefMove) -> String {
    format!(
        "surt: fast-forward {} to surt/{}",
        git::short_branch(&intended.ref_name),
        run_stem(&launch.subject, launch.ordinal)
    )
}

fn merged(intended: &RefMove) -> BaseMerge {
    BaseMerge::Merged {
        from: intended.old_sha.clone(),
        to: intended.new_sha.clone(),
    }
}

fn base_moved(found: Option<String>) -> BaseMerge {
    BaseMerge::Skipped(NotMerged::BaseMoved { found })
}

fn recorded(settled: BaseMerge) -> Finished {
    Finished {
        outcome: settled.outcome(),
        outputs: Vec::new(),
        merged: None,
        recorded: Some(settled),
    }
}
