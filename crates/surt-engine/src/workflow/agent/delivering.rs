use std::collections::{BTreeMap, BTreeSet};

use super::{AgentContract, OutputSpec};
use crate::workflow::error::Problem;

impl AgentContract {
    pub(crate) fn delivering(&self) -> BTreeMap<String, Vec<String>> {
        let outcomes = self.outcomes();
        self.outputs
            .iter()
            .map(|(name, output)| (name.clone(), output.delivering(&outcomes)))
            .collect()
    }

    pub(super) fn check_output_outcomes(&self) -> Result<(), Problem> {
        let outcomes = self.outcomes();
        self.outputs
            .iter()
            .try_for_each(|(name, output)| output.check_outcomes(name, &outcomes))
    }
}

impl OutputSpec {
    pub(super) fn delivers(&self, outcome: &str) -> bool {
        self.outcomes
            .as_ref()
            .is_none_or(|named| named.iter().any(|named| named == outcome))
    }

    fn delivering(&self, outcomes: &[String]) -> Vec<String> {
        outcomes
            .iter()
            .filter(|outcome| self.delivers(outcome))
            .cloned()
            .collect()
    }

    fn check_outcomes(&self, name: &str, declared: &[String]) -> Result<(), Problem> {
        let Some(named) = &self.outcomes else {
            return Ok(());
        };
        let output = name.to_owned();
        let unknown = named.iter().find(|outcome| !declared.contains(outcome));
        match (named.is_empty(), unknown, first_repeat(named)) {
            (true, ..) => Err(Problem::EmptyOutputOutcomes { output }),
            (_, Some(outcome), _) => Err(Problem::UnknownOutputOutcome {
                output,
                outcome: outcome.clone(),
            }),
            (_, _, Some(outcome)) => Err(Problem::DuplicateOutputOutcome {
                output,
                outcome: outcome.clone(),
            }),
            _ => Ok(()),
        }
    }
}

fn first_repeat(names: &[String]) -> Option<&String> {
    let mut seen = BTreeSet::new();
    names.iter().find(|name| !seen.insert(name.as_str()))
}
