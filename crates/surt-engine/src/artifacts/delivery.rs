mod contract;
mod envelope;
mod files;
mod item_list;
mod portable;
mod rejection;

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use envelope::Envelope;

pub(crate) use contract::{DeliveryContract, OutputRule};
pub(crate) use item_list::{Item as ListedItem, ItemListProblem};
pub(crate) use rejection::DeliveryRejection;

#[derive(Debug)]
pub(crate) struct AcceptedDelivery {
    pub(crate) outcome: String,
    pub(crate) outputs: BTreeMap<String, ValidatedOutput>,
}

#[derive(Debug)]
pub(crate) struct ValidatedOutput {
    pub(crate) files: Vec<DeliveredFile>,
    pub(crate) json: Option<Value>,
}

#[derive(Debug)]
pub(crate) struct DeliveredFile {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
}

impl DeliveryContract {
    pub(crate) fn validate(
        &self,
        delivery_dir: &Path,
        envelope: Option<&str>,
    ) -> Result<AcceptedDelivery, DeliveryRejection> {
        let envelope = envelope::parse(envelope)?;
        self.check_names(&envelope)?;
        let outputs = envelope
            .outputs
            .iter()
            .map(|(name, path)| {
                let rule = &self.outputs[name];
                Ok((name.clone(), rule.validate(delivery_dir, path)?))
            })
            .collect::<Result<_, DeliveryRejection>>()?;
        Ok(AcceptedDelivery {
            outcome: envelope.outcome,
            outputs,
        })
    }

    fn check_names(&self, envelope: &Envelope) -> Result<(), DeliveryRejection> {
        let outcome = &envelope.outcome;
        if !self.outcomes.contains(outcome) {
            return Err(DeliveryRejection::UnknownOutcome {
                outcome: outcome.clone(),
            });
        }
        let mut expected = self
            .outputs
            .keys()
            .filter(|name| self.delivers(name, outcome));
        if let Some(name) = expected.find(|name| !envelope.outputs.contains_key(*name)) {
            return Err(DeliveryRejection::MissingOutput { name: name.clone() });
        }
        let mut delivered = envelope.outputs.keys();
        match delivered.find(|name| !self.delivers(name, outcome)) {
            Some(name) => Err(DeliveryRejection::UndeclaredOutput { name: name.clone() }),
            None => Ok(()),
        }
    }

    fn delivers(&self, output: &str, outcome: &str) -> bool {
        self.delivering
            .get(output)
            .is_some_and(|outcomes| outcomes.iter().any(|delivering| delivering == outcome))
    }
}
