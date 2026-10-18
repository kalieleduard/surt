use serde_json::Value;

use super::driver::Driver;
use super::failure::{Failure, OperationError};
use super::launch::Launch;
use crate::artifacts::AcceptedDelivery;
use crate::runtime::AgentRuntime;
use crate::workflow::DeliveryCheck;
use crate::worktree::{self, Claim, EvidenceError};

const INVALID_DELIVERY: &str = "invalid_delivery";
const SHA_FIELD: &str = "sha";

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) async fn prove(
        &self,
        launch: &Launch,
        delivery: &AcceptedDelivery,
    ) -> Result<(), OperationError> {
        for (output, check) in &launch.checks {
            match check {
                DeliveryCheck::CommitEvidence => prove_commit(launch, output, delivery).await?,
            }
        }
        Ok(())
    }
}

async fn prove_commit(
    launch: &Launch,
    output: &str,
    delivery: &AcceptedDelivery,
) -> Result<(), Failure> {
    let invalid =
        |detail: String| Failure::retryable(INVALID_DELIVERY, format!("{output}: {detail}"));
    let sha = delivery
        .outputs
        .get(output)
        .and_then(|delivered| delivered.json.as_ref())
        .and_then(|json| json.get(SHA_FIELD))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("has no {SHA_FIELD} naming a commit")))?;
    let located = launch.worktree.as_ref().and_then(|branch| {
        branch
            .worktree_path
            .as_deref()
            .map(|worktree| (branch, worktree))
    });
    let Some((branch, worktree)) = located else {
        return Err(invalid("the run has no worktree to prove it in".to_owned()));
    };
    let claim = Claim {
        worktree,
        branch: &branch.ref_name,
        start: &branch.start_sha,
        previous: &branch.tip_sha,
        sha,
    };
    worktree::prove(&claim).await.map_err(|error| match error {
        EvidenceError::Unproven(unproven) => invalid(unproven.to_string()),
        EvidenceError::Git(error) => Failure::from(error),
    })
}
