mod preparation;

use super::branches::{CONFIRMED, DISCARDED, moved, run_stem};
use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::operate::OperationLaunch;
use crate::git::{self, worktrees};
use crate::reconcile::worktree::{self, PrepareError, Target};
use crate::runtime::AgentRuntime;
use crate::store::{RunBranch, StartedExecution, StepRow};

const DONE: &str = "done";
const SALVAGE_REFS: &str = "refs/surt/salvage/";

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn create_child_worktree(
        &self,
        launch: &OperationLaunch,
    ) -> Result<&'static str, OperationError> {
        let (recorded, recovered) = {
            let _serialized = self.branches.lock().await;
            match self.driving.store.run_branch(&launch.row.run).await? {
                Some(recorded) => (recorded, true),
                None => (self.intend_child_branch(launch).await?, false),
            }
        };
        if !recovered {
            self.prepare_worktree(&launch.row, &launch.started, &recorded)
                .await?;
            return Ok(DONE);
        }
        let found = git::commit_sha(self.driving.layout.root(), &recorded.ref_name).await?;
        let prepared = self
            .prepare_worktree(&launch.row, &launch.started, &recorded)
            .await;
        let kind = prepared.as_ref().map_or(DISCARDED, |()| CONFIRMED);
        self.settle_intention(&launch.started.id, kind, &recorded, found.as_deref())
            .await?;
        prepared.map(|()| DONE)
    }

    async fn intend_child_branch(
        &self,
        launch: &OperationLaunch,
    ) -> Result<RunBranch, OperationError> {
        let store = self.driving.store;
        let key = launch.row.item.as_deref().ok_or_else(|| {
            Failure::not_retryable(
                "not_a_child_run",
                "only an item's run has a worktree".into(),
            )
        })?;
        let integration = store.run_branch(&launch.root).await?.ok_or_else(|| {
            Failure::not_retryable(
                "no_integration_branch",
                "the root run has no integration branch".into(),
            )
        })?;
        self.require_owned(&integration).await?;
        let worktrees = self.driving.worktrees.as_deref().ok_or_else(|| {
            Failure::not_retryable(
                "no_home_directory",
                "surt cannot find your home directory to place worktrees in".into(),
            )
        })?;
        let stem = run_stem(&launch.subject, launch.ordinal);
        let intended = RunBranch {
            ref_name: self
                .branch_reference(&format!("surt/item/{stem}/{key}"))
                .await?,
            start_sha: integration.tip_sha.clone(),
            tip_sha: integration.tip_sha,
            worktree_path: Some(worktrees.join(stem).join(key)),
        };
        store.record_run_branch(&launch.row.run, &intended).await?;
        Ok(intended)
    }

    pub(super) async fn require_owned(
        &self,
        integration: &RunBranch,
    ) -> Result<(), OperationError> {
        let root = self.driving.layout.root();
        let found = git::commit_sha(root, &integration.ref_name).await?;
        if found.as_deref() != Some(integration.tip_sha.as_str()) {
            return Err(moved(integration, found.as_deref().unwrap_or("no branch")).into());
        }
        let listed = worktrees::list(root).await?;
        let checked_out = listed
            .iter()
            .find(|entry| entry.branch.as_deref() == Some(integration.ref_name.as_str()));
        if let Some(entry) = checked_out {
            return Err(Failure::not_retryable(
                "integration_branch_checked_out",
                format!(
                    "{} is checked out in {}",
                    git::short_branch(&integration.ref_name),
                    entry.path.display()
                ),
            )
            .into());
        }
        Ok(())
    }

    pub(super) async fn prepare_worktree(
        &self,
        row: &StepRow,
        started: &StartedExecution,
        branch: &RunBranch,
    ) -> Result<(), OperationError> {
        let Some(path) = branch.worktree_path.as_deref() else {
            return Ok(());
        };
        let store = self.driving.store;
        let previous = store.previous_execution(&row.run, &started.id).await?;
        let salvaged = previous
            .as_ref()
            .map_or(&started.id, |previous| &previous.id);
        let salvage_ref = format!("{SALVAGE_REFS}{salvaged}");
        let target = Target {
            repository: self.driving.layout.root(),
            branch: &branch.ref_name,
            tip: &branch.tip_sha,
            path,
            owner: &row.run,
            salvage_ref: &salvage_ref,
        };
        let prepared = {
            let _git = self.git_lock().await?;
            worktree::prepare(&target).await.map_err(failure)?
        };
        self.record_preparation(row, started, path, previous.as_ref(), prepared)
            .await
    }
}

fn failure(error: PrepareError) -> Failure {
    match error {
        PrepareError::Foreign(_) => {
            Failure::not_retryable("worktree_path_foreign", error.to_string())
        }
        PrepareError::Git(error) => Failure::from(error),
        PrepareError::Io { .. } | PrepareError::Unverified { .. } => {
            Failure::retryable("worktree_unprepared", error.to_string())
        }
    }
}
