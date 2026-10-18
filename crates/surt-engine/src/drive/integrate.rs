use serde_json::{Value, json};

use super::branches::{moved, run_stem};
use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::operate::{Finished, OperationLaunch};
use crate::checks::CheckReport;
use crate::git::{self, CommitTree, MergeTree};
use crate::project::Error;
use crate::reconcile::{Found, find};
use crate::runtime::AgentRuntime;
use crate::store::{PublishedOutput, RefMove, RunBranch};

const FEEDBACK_INPUT: &str = "feedback";
const REVIEWED_COMMIT_FIELD: &str = "reviewed_commit";
const MERGED: &str = "merged";
const CONFLICT: &str = "conflict";
const CONFLICT_FILE: &str = "conflict.json";

struct Integration {
    task: RunBranch,
    into: RunBranch,
    reviewed: String,
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn integrate(
        &self,
        launch: &OperationLaunch,
    ) -> Result<Finished, OperationError> {
        let store = self.driving.store;
        let into = store.run_branch(&launch.root).await?.ok_or_else(|| {
            Failure::not_retryable(
                "no_integration_branch",
                "the root run has no integration branch".into(),
            )
        })?;
        if let Some(intended) = store.ref_move(&launch.row.id).await?
            && let Some(confirmed) = self.settle(launch, &into, intended).await?
        {
            return Ok(confirmed);
        }
        self.require_owned(&into).await?;
        let integration = Integration {
            task: store.run_branch(&launch.row.run).await?.ok_or_else(|| {
                Failure::not_retryable("no_task_branch", "the task has no branch".into())
            })?,
            reviewed: self
                .input_text(&launch.started, FEEDBACK_INPUT, REVIEWED_COMMIT_FIELD)
                .await?
                .ok_or_else(|| {
                    Failure::not_retryable(
                        "invalid_input",
                        "the feedback input names no reviewed commit".into(),
                    )
                })?,
            into,
        };
        self.merge(launch, &integration).await
    }

    async fn settle(
        &self,
        launch: &OperationLaunch,
        into: &RunBranch,
        intended: RefMove,
    ) -> Result<Option<Finished>, OperationError> {
        let store = self.driving.store;
        match find(self.driving.layout.root(), &intended).await? {
            Found::Moved => Ok(Some(merged(intended.new_sha))),
            Found::NotMoved => {
                store.forget_ref_move(&launch.row.id).await?;
                Ok(None)
            }
            Found::Elsewhere(found) => {
                store.forget_ref_move(&launch.row.id).await?;
                Err(moved(into, found.as_deref().unwrap_or("no branch")).into())
            }
        }
    }

    async fn merge(
        &self,
        launch: &OperationLaunch,
        integration: &Integration,
    ) -> Result<Finished, OperationError> {
        let root = self.driving.layout.root();
        let tip = &integration.into.tip_sha;
        let reviewed = &integration.reviewed;
        let merged_tree = {
            let _git = self.git_lock().await?;
            git::merge_tree(root, tip, reviewed).await?
        };
        let tree = match merged_tree {
            MergeTree::Clean { tree } => tree,
            MergeTree::Conflicted { paths } => {
                let conflict = json!({
                    "kind": "merge",
                    "integration_tip": tip,
                    "reviewed_commit": reviewed,
                    "paths": paths,
                });
                return conflicted(&conflict);
            }
        };
        let merge = self.commit_merge(launch, integration, &tree).await?;
        if *tip != integration.task.start_sha {
            let report = self.recheck(launch, &integration.task, &merge).await?;
            if !report.passed() {
                let conflict = json!({
                    "kind": "checks",
                    "integration_tip": tip,
                    "reviewed_commit": reviewed,
                    "merge_commit": merge,
                    "checks": report.checks(),
                });
                return conflicted(&conflict);
            }
        }
        let intended = RefMove {
            ref_name: integration.into.ref_name.clone(),
            old_sha: tip.clone(),
            new_sha: merge,
        };
        self.move_tip(launch, intended).await
    }

    async fn move_tip(
        &self,
        launch: &OperationLaunch,
        intended: RefMove,
    ) -> Result<Finished, OperationError> {
        self.driving
            .store
            .intend_ref_move(&launch.row.id, &intended)
            .await?;
        {
            let _git = self.git_lock().await?;
            let root = self.driving.layout.root();
            let RefMove {
                ref_name,
                old_sha,
                new_sha,
            } = &intended;
            git::move_ref(root, ref_name, new_sha, old_sha).await?;
        }
        Ok(merged(intended.new_sha))
    }

    async fn commit_merge(
        &self,
        launch: &OperationLaunch,
        integration: &Integration,
        tree: &str,
    ) -> Result<String, OperationError> {
        let started_at = self
            .driving
            .store
            .step_started_at(&launch.row.id)
            .await?
            .ok_or_else(|| Error::internal(NotStarted))?;
        let task = launch.row.item.as_deref().unwrap_or_default();
        let message = format!(
            "Merge task {task} into {}\n\nSurt-Run: {}\nSurt-Task: {task}\nSurt-Reviewed-Commit: {}",
            git::short_branch(&integration.into.ref_name),
            run_stem(&launch.subject, launch.ordinal),
            integration.reviewed
        );
        let commit = CommitTree {
            tree,
            parents: [&integration.into.tip_sha, &integration.reviewed],
            message: &message,
            unix_time: started_at,
        };
        let _git = self.git_lock().await?;
        Ok(git::commit_tree(self.driving.layout.root(), &commit).await?)
    }

    async fn recheck(
        &self,
        launch: &OperationLaunch,
        task: &RunBranch,
        merge: &str,
    ) -> Result<CheckReport, OperationError> {
        let worktree = task.worktree_path.as_deref().ok_or_else(|| {
            Failure::not_retryable("no_worktree", "the task has no worktree".into())
        })?;
        self.prepare_worktree(&launch.row, &launch.started, task)
            .await?;
        git::run(worktree, &["checkout", "--quiet", "--detach", merge]).await?;
        let report = self.run_frozen_checks(launch, worktree, merge).await?;
        let branch = git::short_branch(&task.ref_name);
        git::run(worktree, &["checkout", "--quiet", "--force", branch]).await?;
        Ok(report)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("the integrate step has no start time")]
struct NotStarted;

fn merged(merge: String) -> Finished {
    Finished {
        outcome: MERGED,
        outputs: Vec::new(),
        merged: Some(merge),
    }
}

fn conflicted(conflict: &Value) -> Result<Finished, OperationError> {
    let bytes = serde_json::to_vec_pretty(conflict).map_err(Error::internal)?;
    Ok(Finished {
        outcome: CONFLICT,
        outputs: vec![PublishedOutput {
            name: CONFLICT.to_owned(),
            files: vec![(CONFLICT_FILE.to_owned(), bytes)],
            working_copy: false,
        }],
        merged: None,
    })
}
