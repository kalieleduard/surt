use std::collections::BTreeMap;

use serde::Deserialize;

use super::DeliveryRejection;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope {
    pub(super) outcome: String,
    pub(super) outputs: BTreeMap<String, String>,
}

pub(super) fn parse(text: Option<&str>) -> Result<Envelope, DeliveryRejection> {
    let text = text.ok_or(DeliveryRejection::MissingEnvelope)?;
    serde_json::from_str(text).map_err(|_| DeliveryRejection::InvalidEnvelope)
}
