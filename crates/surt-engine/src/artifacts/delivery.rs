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

pub(crate) use contract::{ConsistencyCheck, DeliveryContract};
pub(crate) use item_list::ItemListProblem;
pub(crate) use rejection::DeliveryRejection;

#[derive(Debug)]
pub(crate) struct AcceptedDelivery {
    pub(crate) outcome: String,
    pub(crate) outputs: BTreeMap<String, ValidatedOutput>,
}

#[derive(Debug)]
pub(crate) struct ValidatedOutput {
    pub(crate) files: Vec<DeliveredFile>,
    #[cfg_attr(
        not(all(test, feature = "test-support")),
        expect(
            dead_code,
            reason = "consistency checks read validated JSON from ticket 13 on"
        )
    )]
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
        let delivery = AcceptedDelivery {
            outcome: envelope.outcome,
            outputs,
        };
        self.check_consistency(&delivery)?;
        Ok(delivery)
    }

    fn check_consistency(&self, delivery: &AcceptedDelivery) -> Result<(), DeliveryRejection> {
        self.checks.iter().try_for_each(|registered| {
            (registered.check)(delivery).map_err(|detail| DeliveryRejection::Inconsistent {
                check: registered.name.to_owned(),
                detail,
            })
        })
    }

    fn check_names(&self, envelope: &Envelope) -> Result<(), DeliveryRejection> {
        if !self.outcomes.contains(&envelope.outcome) {
            return Err(DeliveryRejection::UnknownOutcome {
                outcome: envelope.outcome.clone(),
            });
        }
        if let Some(name) = self
            .outputs
            .keys()
            .find(|name| !envelope.outputs.contains_key(*name))
        {
            return Err(DeliveryRejection::MissingOutput { name: name.clone() });
        }
        match envelope
            .outputs
            .keys()
            .find(|name| !self.outputs.contains_key(*name))
        {
            Some(name) => Err(DeliveryRejection::UndeclaredOutput { name: name.clone() }),
            None => Ok(()),
        }
    }
}
