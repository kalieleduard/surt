use std::collections::BTreeMap;
use std::error::Error;

use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::Value;

use super::references::local_references;

const SCHEMA_BASE: &str = "https://surt.invalid/";

struct AgentSchemas(BTreeMap<String, Value>);

impl Retrieve for AgentSchemas {
    fn retrieve(&self, uri: &Uri<String>) -> Result<Value, Box<dyn Error + Send + Sync>> {
        let path = uri.as_str().strip_prefix(SCHEMA_BASE).unwrap_or_default();
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| format!("{path} is not frozen with the agent").into())
    }
}

pub(crate) fn compile_schema<'a>(
    path: &str,
    text: &str,
    schema_text: &impl Fn(&str) -> Option<&'a str>,
) -> Result<Validator, String> {
    let schema: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    jsonschema::options()
        .with_draft(Draft::Draft7)
        .with_base_uri(format!("{SCHEMA_BASE}{path}"))
        .with_retriever(AgentSchemas(referenced(path, text, schema_text)))
        .build(&schema)
        .map_err(|error| error.to_string())
}

fn referenced<'a>(
    path: &str,
    text: &str,
    schema_text: &impl Fn(&str) -> Option<&'a str>,
) -> BTreeMap<String, Value> {
    let mut schemas = BTreeMap::new();
    let mut pending = local_references(path, text);
    while let Some(path) = pending.pop() {
        if schemas.contains_key(&path) {
            continue;
        }
        let Some(text) = schema_text(&path) else {
            continue;
        };
        let Ok(schema) = serde_json::from_str(text) else {
            continue;
        };
        pending.extend(local_references(&path, text));
        schemas.insert(path, schema);
    }
    schemas
}
