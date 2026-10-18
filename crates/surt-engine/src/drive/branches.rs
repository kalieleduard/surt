use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::operate::OperationLaunch;
use crate::git;
use crate::runtime::AgentRuntime;
use crate::store::RunBranch;

const DONE: &str = "done";
const BASE_INPUT: &str = "base";
const SHA_FIELD: &str = "sha";

pub(super) fn run_stem(subject: &str, ordinal: u32) -> String {
    if ordinal > 1 {
        format!("{subject}.{ordinal}")
    } else {
        subject.to_owned()
    }
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn create_run_branch(
        &self,
        launch: &OperationLaunch,
    ) -> Result<&'static str, OperationError> {
        let base = self.input_sha(launch, BASE_INPUT).await?;
        let branch = format!("surt/{}", run_stem(&launch.subject, launch.ordinal));
        let reference = self.branch_reference(&branch).await?;
        let store = self.driving.store;
        let recorded = match store.run_branch(&launch.row.run).await? {
            Some(recorded) => recorded,
            None => {
                let intended = RunBranch {
                    ref_name: reference,
                    start_sha: base.clone(),
                    tip_sha: base,
                    worktree_path: None,
                };
                store.record_run_branch(&launch.row.run, &intended).await?;
                intended
            }
        };
        let _git = self.git_lock().await?;
        let root = self.driving.layout.root();
        match git::commit_sha(root, &recorded.ref_name).await? {
            None => git::create_ref(root, &recorded.ref_name, &recorded.tip_sha).await?,
            Some(found) if found == recorded.tip_sha => {}
            Some(found) => return Err(moved(&recorded, &found).into()),
        }
        Ok(DONE)
    }

    pub(super) async fn branch_reference(&self, branch: &str) -> Result<String, OperationError> {
        let root = self.driving.layout.root();
        if !git::is_valid_branch_name(root, branch).await? {
            return Err(Failure::not_retryable(
                "invalid_branch_name",
                format!("{branch} is not a valid branch name"),
            )
            .into());
        }
        Ok(format!("{}{branch}", git::BRANCH_NAMESPACE))
    }

    pub(super) async fn input_sha(
        &self,
        launch: &OperationLaunch,
        input: &str,
    ) -> Result<String, OperationError> {
        self.input_text(&launch.started, input, SHA_FIELD)
            .await?
            .ok_or_else(|| {
                Failure::not_retryable(
                    "invalid_input",
                    format!("the {input} input has no readable commit"),
                )
                .into()
            })
    }
}

pub(super) fn moved(recorded: &RunBranch, found: &str) -> Failure {
    let branch = git::short_branch(&recorded.ref_name);
    Failure::not_retryable(
        "integration_branch_moved",
        format!(
            "{branch} moved outside surt (expected {}, found {found})",
            recorded.tip_sha
        ),
    )
}
