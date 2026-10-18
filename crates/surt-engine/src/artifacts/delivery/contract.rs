use std::collections::BTreeMap;

use jsonschema::Validator;
use serde_json::Value;

use super::AcceptedDelivery;
use crate::workflow::{AgentContract, Format, ItemFileSpec, OutputSpec};

#[derive(Debug)]
pub(crate) struct DeliveryContract {
    pub(super) outcomes: Vec<String>,
    pub(super) outputs: BTreeMap<String, OutputRule>,
    pub(super) checks: Vec<ConsistencyCheck>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ConsistencyCheck {
    pub(crate) name: &'static str,
    pub(crate) check: fn(&AcceptedDelivery) -> Result<(), String>,
}

#[derive(Debug)]
pub(crate) struct OutputRule {
    pub(super) file: FileRule,
    pub(super) item_files: BTreeMap<String, FileRule>,
}

#[derive(Debug)]
pub(super) struct FileRule {
    pub(super) format: Format,
    pub(super) schema: Option<Validator>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("schema {path} cannot be compiled: {detail}")]
pub(crate) struct InvalidSchema {
    pub(crate) path: String,
    pub(crate) detail: String,
}

impl DeliveryContract {
    pub(crate) fn new<'a>(
        agent: &AgentContract,
        schema_text: impl Fn(&str) -> Option<&'a str>,
    ) -> Result<DeliveryContract, InvalidSchema> {
        let outputs = agent
            .outputs
            .iter()
            .map(|(name, spec)| Ok((name.clone(), OutputRule::new(spec, &schema_text)?)))
            .collect::<Result<_, InvalidSchema>>()?;
        Ok(DeliveryContract {
            outcomes: agent.outcomes(),
            outputs,
            checks: Vec::new(),
        })
    }

    pub(crate) fn with_check(mut self, check: ConsistencyCheck) -> DeliveryContract {
        self.checks.push(check);
        self
    }

    pub(crate) fn output(&self, name: &str) -> Option<&OutputRule> {
        self.outputs.get(name)
    }
}

impl OutputRule {
    fn new<'a>(
        spec: &OutputSpec,
        schema_text: &impl Fn(&str) -> Option<&'a str>,
    ) -> Result<OutputRule, InvalidSchema> {
        let item_files = spec
            .item_files
            .iter()
            .map(|(name, file)| Ok((name.clone(), FileRule::for_item_file(file, schema_text)?)))
            .collect::<Result<_, InvalidSchema>>()?;
        Ok(OutputRule {
            file: FileRule::new(spec.format, spec.json_schema.as_deref(), schema_text)?,
            item_files,
        })
    }
}

impl FileRule {
    fn for_item_file<'a>(
        spec: &ItemFileSpec,
        schema_text: &impl Fn(&str) -> Option<&'a str>,
    ) -> Result<FileRule, InvalidSchema> {
        FileRule::new(spec.format, spec.json_schema.as_deref(), schema_text)
    }

    fn new<'a>(
        format: Format,
        schema_path: Option<&str>,
        schema_text: &impl Fn(&str) -> Option<&'a str>,
    ) -> Result<FileRule, InvalidSchema> {
        let schema = schema_path
            .map(|path| compile(path, schema_text))
            .transpose()?;
        Ok(FileRule { format, schema })
    }
}

fn compile<'a>(
    path: &str,
    schema_text: &impl Fn(&str) -> Option<&'a str>,
) -> Result<Validator, InvalidSchema> {
    let invalid = |detail: String| InvalidSchema {
        path: path.to_owned(),
        detail,
    };
    let text = schema_text(path).ok_or_else(|| invalid("missing".to_owned()))?;
    let schema: Value = serde_json::from_str(text).map_err(|error| invalid(error.to_string()))?;
    jsonschema::draft7::new(&schema).map_err(|error| invalid(error.to_string()))
}
