use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

pub(super) fn envelope_schema(
    outcomes: &[String],
    outputs: &BTreeMap<String, Vec<String>>,
) -> String {
    let properties: Map<String, Value> = outputs
        .keys()
        .map(|name| (name.clone(), json!({ "type": "string" })))
        .collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["outcome", "outputs"],
        "properties": {
            "outcome": { "enum": outcomes },
            "outputs": {
                "type": "object",
                "additionalProperties": false,
                "required": delivered_on_every(outcomes, outputs),
                "properties": properties
            }
        }
    })
    .to_string()
}

fn delivered_on_every<'a>(
    outcomes: &[String],
    outputs: &'a BTreeMap<String, Vec<String>>,
) -> Vec<&'a String> {
    outputs
        .iter()
        .filter(|(_, delivering)| outcomes.iter().all(|outcome| delivering.contains(outcome)))
        .map(|(name, _)| name)
        .collect()
}
