use std::path::Path;

use tokio::sync::mpsc;

use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::operate::{Finished, OperationLaunch};
use crate::operations::checks::{self, CheckReport};
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::PublishedOutput;

const COMMIT_INPUT: &str = "commit";
const CHECK_REPORT: &str = "check-report";
const REPORT_FILE: &str = "check-report.json";
const OUTPUT_LOG: &str = "output.log";
const PASSED: &str = "passed";
const FAILED: &str = "failed";

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn run_checks(
        &self,
        launch: &OperationLaunch,
    ) -> Result<Finished, OperationError> {
        let commit = self.input_sha(launch, COMMIT_INPUT).await?;
        let located = self
            .driving
            .store
            .run_branch(&launch.row.run)
            .await?
            .and_then(|branch| branch.worktree_path.clone().map(|path| (branch, path)));
        let Some((branch, worktree)) = located else {
            return Err(Failure::not_retryable(
                "no_worktree",
                "checks run in a child run's worktree".into(),
            )
            .into());
        };
        self.prepare_worktree(&launch.row, &launch.started, &branch)
            .await?;
        if branch.tip_sha != commit {
            return Err(Failure::not_retryable(
                "commit_not_at_tip",
                format!("{commit} is not the accepted tip {}", branch.tip_sha),
            )
            .into());
        }
        let report = self.run_frozen_checks(launch, &worktree, &commit).await?;
        let outcome = if report.passed() { PASSED } else { FAILED };
        let bytes = serde_json::to_vec_pretty(&report).map_err(Error::internal)?;
        Ok(Finished {
            outcome,
            outputs: vec![PublishedOutput {
                name: CHECK_REPORT.to_owned(),
                files: vec![(REPORT_FILE.to_owned(), bytes)],
                working_copy: false,
            }],
            merged: None,
        })
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn run_frozen_checks(
        &self,
        launch: &OperationLaunch,
        worktree: &Path,
        commit: &str,
    ) -> Result<CheckReport, OperationError> {
        let log = self
            .driving
            .layout
            .execution_logs(&launch.subject, &launch.started.id)
            .join(OUTPUT_LOG);
        let (spawned, pids) = mpsc::unbounded_channel();
        let running = checks::run(&self.frozen.checks, worktree, commit, &log, &spawned);
        let report = self.tracking(&launch.started.id, running, pids).await?;
        report.map_err(|error| {
            Failure::retryable(
                "checks_unrecorded",
                format!("cannot record the checks in {}: {error}", log.display()),
            )
            .into()
        })
    }
}
