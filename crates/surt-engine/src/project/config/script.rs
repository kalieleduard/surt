use serde::{Deserialize, Deserializer, Serialize};

use super::check::{command, named};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    #[serde(deserialize_with = "identifier")]
    pub name: String,
    #[serde(deserialize_with = "argv")]
    pub run: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn identifier<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    named("script", String::deserialize(deserializer)?)
}

fn argv<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    command("script", Vec::<String>::deserialize(deserializer)?)
}
