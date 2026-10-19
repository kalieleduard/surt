mod artifacts;

use super::super::definitions::ProjectDefinitions;
use super::super::{Error, Project, Refusal};
use crate::workflow::{InputFormat, RunInputSpec, RunInputs, validate};

pub(super) use artifacts::build;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredInput {
    pub name: String,
    pub format: InputFormat,
    pub optional: bool,
    pub default: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputProblem {
    Unknown { name: String },
    Duplicate { name: String },
    Missing { name: String },
    Empty { name: String },
    NotJson { name: String, detail: String },
    Unresolved { name: String, reference: String },
}

pub(super) enum Value<'a> {
    Given(&'a str),
    Default(&'a str),
    Carried,
}

pub(super) struct Supplied<'a> {
    pub(super) name: &'a str,
    pub(super) format: InputFormat,
    pub(super) value: Value<'a>,
}

impl Project {
    pub fn subject_input<'a>(
        &self,
        workflow: &str,
        inputs: &'a [(String, String)],
    ) -> Result<Option<&'a str>, Error> {
        let definitions = ProjectDefinitions::new(&self.layout);
        definitions.require(workflow)?;
        let declared = validate(&definitions, workflow)?;
        check(&declared, inputs, &[]).map_err(|problem| refused(workflow, &declared, problem))?;
        let Some(from) = &declared.subject else {
            return Ok(None);
        };
        Ok(given(inputs, from))
    }
}

pub(super) fn refused(workflow: &str, inputs: &RunInputs, problem: InputProblem) -> Error {
    Refusal::InvalidInput {
        workflow: workflow.to_owned(),
        problem,
        declared: inputs
            .declared
            .iter()
            .map(|(name, spec)| DeclaredInput {
                name: name.to_owned(),
                format: spec.format,
                optional: spec.optional || spec.default.is_some(),
                default: spec.default.clone(),
                description: spec.description.clone(),
            })
            .collect(),
    }
    .into()
}

pub(super) fn check<'a>(
    inputs: &'a RunInputs,
    values: &'a [(String, String)],
    carried: &[String],
) -> Result<Vec<Supplied<'a>>, InputProblem> {
    refuse_unknown(inputs, values)?;
    let mut supplied = Vec::new();
    for (name, spec) in inputs.declared.iter() {
        let Some(value) = value_of(name, spec, values, carried) else {
            if spec.optional {
                continue;
            }
            return Err(InputProblem::Missing {
                name: name.to_owned(),
            });
        };
        if let Value::Given(text) = value {
            check_format(name, spec.format, text)?;
        }
        supplied.push(Supplied {
            name,
            format: spec.format,
            value,
        });
    }
    Ok(supplied)
}

fn refuse_unknown(inputs: &RunInputs, values: &[(String, String)]) -> Result<(), InputProblem> {
    for (index, (name, _)) in values.iter().enumerate() {
        if !inputs.declared.contains(name) {
            return Err(InputProblem::Unknown { name: name.clone() });
        }
        if values[..index].iter().any(|(earlier, _)| earlier == name) {
            return Err(InputProblem::Duplicate { name: name.clone() });
        }
    }
    Ok(())
}

fn value_of<'a>(
    name: &str,
    spec: &'a RunInputSpec,
    values: &'a [(String, String)],
    carried: &[String],
) -> Option<Value<'a>> {
    if let Some(value) = given(values, name) {
        return Some(Value::Given(value));
    }
    if carried.iter().any(|carried| carried == name) {
        return Some(Value::Carried);
    }
    spec.default.as_deref().map(Value::Default)
}

fn given<'a>(values: &'a [(String, String)], name: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|(given, _)| given == name)
        .map(|(_, value)| value.as_str())
}

fn check_format(name: &str, format: InputFormat, value: &str) -> Result<(), InputProblem> {
    match format {
        InputFormat::Text | InputFormat::Markdown if value.trim().is_empty() => {
            Err(InputProblem::Empty {
                name: name.to_owned(),
            })
        }
        InputFormat::Json => serde_json::from_str::<serde_json::Value>(value)
            .map(|_| ())
            .map_err(|error| InputProblem::NotJson {
                name: name.to_owned(),
                detail: error.to_string(),
            }),
        _ => Ok(()),
    }
}
