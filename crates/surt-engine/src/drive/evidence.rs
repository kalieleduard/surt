use serde_json::Value;

use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::inputs::text;
use super::launch::Launch;
use crate::artifacts::AcceptedDelivery;
use crate::git::GitError;
use crate::reconcile::worktree::{self, Claim, EvidenceError};
use crate::runtime::AgentRuntime;
use crate::store::StoreError;
use crate::workflow::DeliveryCheck;

const INVALID_DELIVERY: &str = "invalid_delivery";
const SHA_FIELD: &str = "sha";
const COMMIT_INPUT: &str = "commit";
const CHECK_REPORT_INPUT: &str = "check-report";
const REPORTED_COMMIT_FIELD: &str = "commit";
const REVIEWED_COMMIT_FIELD: &str = "reviewed_commit";
const FINDINGS_FIELD: &str = "findings";
const BLOCKING_FIELD: &str = "blocking";
const CONFLICT_INPUT: &str = "conflict";
const INTEGRATION_TIP_FIELD: &str = "integration_tip";

enum Disproof {
    Invalid(String),
    Failed(OperationError),
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn prove(
        &self,
        launch: &Launch,
        delivery: &AcceptedDelivery,
    ) -> Result<(), OperationError> {
        for (output, check) in &launch.checks {
            let Some(delivered) = delivery.outputs.get(output) else {
                continue;
            };
            let json = delivered.json.as_ref();
            let proven = match check {
                DeliveryCheck::CommitEvidence {} => self.prove_commit(launch, json).await,
                DeliveryCheck::ReviewedCommit {} => self.prove_review(launch, json).await,
                DeliveryCheck::NoBlockingFinding { outcome } => {
                    without_blocking(json, &delivery.outcome, outcome)
                }
            };
            proven.map_err(|disproof| match disproof {
                Disproof::Invalid(detail) => {
                    Failure::retryable(INVALID_DELIVERY, format!("{output}: {detail}")).into()
                }
                Disproof::Failed(error) => error,
            })?;
        }
        Ok(())
    }

    async fn prove_commit(&self, launch: &Launch, json: Option<&Value>) -> Result<(), Disproof> {
        let sha = json
            .and_then(|json| text(json, SHA_FIELD))
            .ok_or_else(|| Disproof::Invalid(format!("has no {SHA_FIELD} naming a commit")))?;
        let located = launch.worktree.as_ref().and_then(|branch| {
            branch
                .worktree_path
                .as_deref()
                .map(|worktree| (branch, worktree))
        });
        let Some((branch, worktree)) = located else {
            return Err(Disproof::Invalid(
                "the run has no worktree to prove it in".to_owned(),
            ));
        };
        let integration_tip = self
            .input_text(&launch.started, CONFLICT_INPUT, INTEGRATION_TIP_FIELD)
            .await?;
        let claim = Claim {
            worktree,
            branch: &branch.ref_name,
            start: &branch.start_sha,
            previous: &branch.tip_sha,
            integration_tip: integration_tip.as_deref(),
            sha: &sha,
        };
        worktree::prove(&claim).await.map_err(|error| match error {
            EvidenceError::Unproven(unproven) => Disproof::Invalid(unproven.to_string()),
            EvidenceError::Git(error) => Disproof::from(error),
        })
    }

    async fn prove_review(&self, launch: &Launch, json: Option<&Value>) -> Result<(), Disproof> {
        let reviewed = json
            .and_then(|json| text(json, REVIEWED_COMMIT_FIELD))
            .ok_or_else(|| Disproof::Invalid(format!("has no {REVIEWED_COMMIT_FIELD}")))?;
        let started = &launch.started;
        let commit = self
            .input_text(started, COMMIT_INPUT, SHA_FIELD)
            .await?
            .ok_or_else(|| Disproof::Invalid("no commit is under review".to_owned()))?;
        if reviewed != commit {
            return Err(Disproof::Invalid(format!(
                "{REVIEWED_COMMIT_FIELD} {reviewed} is not the commit under review {commit}"
            )));
        }
        let reported = self
            .input_text(started, CHECK_REPORT_INPUT, REPORTED_COMMIT_FIELD)
            .await?;
        match reported {
            Some(reported) if reported != commit => Err(Disproof::Invalid(format!(
                "the {CHECK_REPORT_INPUT} is for {reported}, not for the commit under review {commit}"
            ))),
            _ => Ok(()),
        }
    }
}

fn without_blocking(json: Option<&Value>, delivered: &str, outcome: &str) -> Result<(), Disproof> {
    let blocked = json
        .and_then(|json| json.get(FINDINGS_FIELD))
        .and_then(Value::as_array)
        .is_some_and(|findings| {
            findings
                .iter()
                .any(|finding| finding.get(BLOCKING_FIELD) == Some(&Value::Bool(true)))
        });
    if delivered == outcome && blocked {
        return Err(Disproof::Invalid(format!(
            "{outcome} with a blocking finding"
        )));
    }
    Ok(())
}

impl From<StoreError> for Disproof {
    fn from(error: StoreError) -> Disproof {
        Disproof::Failed(error.into())
    }
}

impl From<GitError> for Disproof {
    fn from(error: GitError) -> Disproof {
        Disproof::Failed(error.into())
    }
}
