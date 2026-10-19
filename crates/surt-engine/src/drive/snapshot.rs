use serde_json::{Value, json};

use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::inputs::text;
use super::operate::{Finished, OperationLaunch};
use crate::git;
use crate::project::Error;
use crate::runtime::AgentRuntime;
use crate::store::PublishedOutput;

const BASE_INPUT: &str = "base";
const SHA_FIELD: &str = "sha";
const DONE: &str = "done";
const INTEGRATION: &str = "integration";
const INTEGRATION_FILE: &str = "integration.json";
const SECOND_PARENT: &str = "^2";

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn snapshot_run_branch(
        &self,
        launch: &OperationLaunch,
    ) -> Result<Finished, OperationError> {
        let base = self.input_json(&launch.started, BASE_INPUT).await?;
        let base_sha = base
            .as_ref()
            .and_then(|base| text(base, SHA_FIELD))
            .ok_or_else(|| Failure::invalid_input("the base input names no commit".into()))?;
        let branch = self
            .driving
            .store
            .run_branch(&launch.row.run)
            .await?
            .ok_or_else(|| {
                Failure::not_retryable("no_integration_branch", "the run has no branch".into())
            })?;
        self.require_owned(&branch).await?;
        if !git::is_ancestor(self.driving.layout.root(), &base_sha, &branch.tip_sha).await? {
            return Err(Failure::not_retryable(
                "integration_branch_unrelated",
                format!(
                    "{} does not descend from the base {base_sha}",
                    branch.tip_sha
                ),
            )
            .into());
        }
        let integration = json!({
            "branch": git::short_branch(&branch.ref_name),
            "base": base,
            "tip": branch.tip_sha,
            "items": self.merged_items(&launch.row.run).await?,
        });
        let bytes = serde_json::to_vec_pretty(&integration).map_err(Error::internal)?;
        Ok(Finished {
            outcome: DONE,
            outputs: vec![PublishedOutput {
                name: INTEGRATION.to_owned(),
                files: vec![(INTEGRATION_FILE.to_owned(), bytes)],
                working_copy: false,
            }],
            merged: None,
            recorded: None,
        })
    }

    async fn merged_items(&self, run: &str) -> Result<Vec<Value>, OperationError> {
        let mut merged = Vec::new();
        for (key, merge) in self.driving.store.merged_children(run).await? {
            let reviewed = format!("{merge}{SECOND_PARENT}");
            let reviewed = git::commit_sha(self.driving.layout.root(), &reviewed)
                .await?
                .ok_or_else(|| {
                    Failure::not_retryable(
                        "not_a_merge",
                        format!("item {key} was recorded as merged by {merge}, which is no merge"),
                    )
                })?;
            merged.push(json!({ "key": key, "reviewed_commit": reviewed, "merge_commit": merge }));
        }
        Ok(merged)
    }
}
