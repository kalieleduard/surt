use std::collections::{BTreeMap, BTreeSet};

use super::agent::{AgentContract, Format};
use super::definition::{Definition, Step};
use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::steps::{AgentStep, StepSpec};
use super::structure::dependencies;
use crate::operations::Operation;

pub(crate) type Available = BTreeMap<String, Option<Format>>;

pub(crate) type Agents = BTreeMap<String, AgentContract>;

mod external;

use external::child_use;
pub(super) use external::{ChildUse, External};

pub(super) fn check(
    definition: &Definition,
    agents: &Agents,
    external: &External,
) -> Result<Vec<ChildUse>, Box<InvalidDefinition>> {
    let mut children = Vec::new();
    for step in &definition.steps {
        let available = available_at(definition, agents, &external.artifacts, step);
        let checked = match &step.spec {
            StepSpec::Agent(spec) => check_agent(spec, agents, &available),
            StepSpec::Operation(spec) => Operation::find(&spec.run).map_or(Ok(()), |operation| {
                check_operation(operation, external, &available)
            }),
            StepSpec::Approval(spec) => produced(&spec.artifact, &available).map(|_| ()),
            StepSpec::PerItem(spec) => {
                child_use(step, spec, external, &available).map(|child| children.push(child))
            }
            StepSpec::Loop(_) => Ok(()),
        };
        checked.map_err(|problem| {
            InvalidDefinition::new(
                DefinitionFile::Workflow(definition.name.clone()),
                Some(&step.key),
                problem,
            )
        })?;
    }
    Ok(children)
}

fn check_agent(spec: &AgentStep, agents: &Agents, available: &Available) -> Result<(), Problem> {
    let Some(contract) = agents.get(&spec.agent) else {
        return Ok(());
    };
    if let Some(input) = spec
        .inputs
        .keys()
        .find(|input| !contract.inputs.contains_key(*input))
    {
        return Err(Problem::UnknownAgentInput {
            input: input.clone(),
        });
    }
    let required = contract
        .inputs
        .iter()
        .filter(|(_, input)| !input.optional)
        .map(|(name, _)| spec.inputs.get(name).unwrap_or(name).as_str());
    require(required, available)
}

fn check_operation(
    operation: Operation,
    external: &External,
    available: &Available,
) -> Result<(), Problem> {
    require(operation.inputs().iter().copied(), available)?;
    match operation
        .git_inputs()
        .iter()
        .find(|input| !external.git_refs.contains(**input))
    {
        Some(input) => Err(Problem::GitInputNotGitRef {
            input: (*input).to_owned(),
        }),
        None => Ok(()),
    }
}

fn require<'a>(
    mut inputs: impl Iterator<Item = &'a str>,
    available: &Available,
) -> Result<(), Problem> {
    match inputs.find(|input| !available.contains_key(*input)) {
        Some(input) => Err(Problem::MissingInput {
            input: input.to_owned(),
        }),
        None => Ok(()),
    }
}

fn produced(artifact: &str, available: &Available) -> Result<Option<Format>, Problem> {
    available
        .get(artifact)
        .copied()
        .ok_or_else(|| Problem::UnproducedArtifact {
            artifact: artifact.to_owned(),
        })
}

fn available_at(
    definition: &Definition,
    agents: &Agents,
    external: &Available,
    step: &Step,
) -> Available {
    let mut available = external.clone();
    for predecessor in predecessors(definition, step) {
        available.extend(produces(definition, agents, predecessor));
    }
    available
}

fn predecessors<'a>(definition: &'a Definition, step: &'a Step) -> Vec<&'a Step> {
    let mut pending: Vec<&str> = dependencies(step).collect();
    pending.extend(
        definition
            .enclosing_loop(&step.key)
            .into_iter()
            .flat_map(dependencies),
    );
    let mut seen = BTreeSet::new();
    let mut found = Vec::new();
    while let Some(key) = pending.pop() {
        if seen.insert(key)
            && let Some(predecessor) = definition.step(key)
        {
            found.push(predecessor);
            pending.extend(dependencies(predecessor));
        }
    }
    found
}

fn produces(
    definition: &Definition,
    agents: &Agents,
    step: &Step,
) -> Vec<(String, Option<Format>)> {
    match &step.spec {
        StepSpec::Agent(spec) => agents.get(&spec.agent).map_or_else(Vec::new, |contract| {
            contract
                .outputs
                .iter()
                .map(|(name, output)| (name.clone(), Some(output.format)))
                .collect()
        }),
        StepSpec::Operation(spec) => {
            Operation::find(&spec.run).map_or_else(Vec::new, |operation| {
                operation
                    .produces()
                    .iter()
                    .map(|name| ((*name).to_owned(), Some(Format::Json)))
                    .collect()
            })
        }
        StepSpec::Approval(spec) => vec![(spec.rejection_artifact.clone(), Some(Format::Text))],
        StepSpec::Loop(spec) => spec
            .body
            .iter()
            .filter_map(|member| definition.step(member))
            .flat_map(|member| produces(definition, agents, member))
            .collect(),
        StepSpec::PerItem(_) => Vec::new(),
    }
}
