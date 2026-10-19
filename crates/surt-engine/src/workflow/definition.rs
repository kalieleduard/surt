use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::fields;
use super::steps::{StepKind, StepSpec};

const SCHEMA_VERSION: i64 = 1;

pub(crate) struct Definition {
    pub(crate) name: String,
    description: Option<String>,
    cleanup: CleanupPolicy,
    pub(crate) steps: Vec<Step>,
}

pub(crate) struct Step {
    pub(crate) key: String,
    pub(crate) spec: StepSpec,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CleanupPolicy {
    #[serde(default)]
    pub(crate) on_success: OnSuccess,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnSuccess {
    #[default]
    Clean,
    Keep,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: i64,
    name: String,
    description: Option<String>,
    #[serde(default)]
    cleanup: CleanupPolicy,
    steps: toml::Table,
}

#[derive(Serialize)]
struct Canonical<'a> {
    schema: i64,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    cleanup: &'a CleanupPolicy,
    steps: CanonicalSteps<'a>,
}

struct CanonicalSteps<'a>(&'a [Step]);

impl Serialize for CanonicalSteps<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for step in self.0 {
            map.serialize_entry(&step.key, &step.spec)?;
        }
        map.end()
    }
}

impl Definition {
    pub(crate) fn parse(name: &str, text: &str) -> Result<Definition, Box<InvalidDefinition>> {
        let invalid = |step: Option<&str>, problem: Problem| {
            let field = fields::workflow_field(&problem, text);
            InvalidDefinition::new(DefinitionFile::Workflow(name.to_owned()), step, problem)
                .with_field(field)
        };
        let document: Document =
            toml::from_str(text).map_err(|error| invalid(None, Problem::syntax(text, &error)))?;
        if document.schema != SCHEMA_VERSION {
            return Err(invalid(
                None,
                Problem::UnsupportedSchema {
                    found: document.schema,
                },
            ));
        }
        if document.name != name {
            return Err(invalid(
                None,
                Problem::NameMismatch {
                    found: document.name,
                },
            ));
        }
        let steps = document
            .steps
            .into_iter()
            .map(|(key, value)| {
                parse_step(value)
                    .map(|spec| Step {
                        key: key.clone(),
                        spec,
                    })
                    .map_err(|problem| invalid(Some(&key), problem))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Definition {
            name: document.name,
            description: document.description,
            cleanup: document.cleanup,
            steps,
        })
    }

    pub(crate) fn on_success(&self) -> OnSuccess {
        self.cleanup.on_success
    }

    pub(crate) fn step(&self, key: &str) -> Option<&Step> {
        self.steps.iter().find(|step| step.key == key)
    }

    pub(crate) fn enclosing_loop(&self, key: &str) -> Option<&Step> {
        self.steps.iter().find(
            |step| matches!(&step.spec, StepSpec::Loop(spec) if spec.body.iter().any(|member| member == key)),
        )
    }

    pub(crate) fn canonical(&self) -> Result<String, toml::ser::Error> {
        let canonical = Canonical {
            schema: SCHEMA_VERSION,
            name: &self.name,
            description: self.description.as_deref(),
            cleanup: &self.cleanup,
            steps: CanonicalSteps(&self.steps),
        };
        toml::to_string(&canonical)
    }
}

fn parse_step(value: toml::Value) -> Result<StepSpec, Problem> {
    let Some(kind) = value.get("kind").cloned() else {
        return Err(Problem::MissingKind);
    };
    if kind.clone().try_into::<StepKind>().is_err() {
        return Err(Problem::UnknownKind {
            kind: kind
                .as_str()
                .map_or_else(|| kind.to_string(), str::to_owned),
        });
    }
    value
        .try_into::<StepSpec>()
        .map_err(|error| Problem::InvalidStep {
            detail: error.message().to_owned(),
        })
}
