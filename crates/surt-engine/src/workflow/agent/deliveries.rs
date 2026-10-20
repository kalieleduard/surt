use serde::{Deserialize, Serialize};

use super::{AgentContract, Format, OutputSpec};
use crate::workflow::error::Problem;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DeliveryCheck {
    CommitEvidence {},
    ReviewedCommit {},
    NoBlockingFinding { outcome: String },
}

impl AgentContract {
    pub(crate) fn delivery_checks(&self) -> Vec<(String, DeliveryCheck)> {
        self.outputs
            .iter()
            .flat_map(|(name, output)| {
                output
                    .checks
                    .iter()
                    .map(move |check| (name.clone(), check.clone()))
            })
            .collect()
    }

    pub(super) fn check_deliveries(&self) -> Result<(), Problem> {
        let outcomes = self.outcomes();
        self.outputs
            .iter()
            .try_for_each(|(name, output)| output.check_deliveries(name, &outcomes))
    }
}

impl OutputSpec {
    fn check_deliveries(&self, name: &str, outcomes: &[String]) -> Result<(), Problem> {
        if !self.checks.is_empty() && self.format != Format::Json {
            return Err(Problem::MissingCheckedOutput {
                output: name.to_owned(),
            });
        }
        self.checks
            .iter()
            .filter_map(|check| match check {
                DeliveryCheck::NoBlockingFinding { outcome } => Some(outcome),
                _ => None,
            })
            .try_for_each(|outcome| self.check_outcome(name, outcomes, outcome))
    }

    fn check_outcome(&self, name: &str, outcomes: &[String], outcome: &str) -> Result<(), Problem> {
        if !outcomes.iter().any(|declared| declared == outcome) {
            return Err(Problem::MissingCheckedOutcome {
                outcome: outcome.to_owned(),
            });
        }
        if !self.delivers(outcome) {
            return Err(Problem::UndeliveredCheckedOutput {
                output: name.to_owned(),
                outcome: outcome.to_owned(),
            });
        }
        Ok(())
    }
}
