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
        let undeclared = self.checks.iter().find_map(|check| match check {
            DeliveryCheck::NoBlockingFinding { outcome } if !outcomes.contains(outcome) => {
                Some(outcome)
            }
            _ => None,
        });
        match undeclared {
            Some(outcome) => Err(Problem::MissingCheckedOutcome {
                outcome: outcome.clone(),
            }),
            None => Ok(()),
        }
    }
}
