use super::definition::Definition;
use super::error::{DefinitionFile, InvalidDefinition, Problem, StepField};
use super::flow::Agents;
use super::steps::{Condition, StepSpec};
use crate::operations::Operation;

const APPROVAL_OUTCOMES: [&str; 2] = ["approved", "rejected"];
const ENGINE_OUTCOME: &str = "done";

pub(super) fn check(
    definition: &Definition,
    agents: &Agents,
) -> Result<(), Box<InvalidDefinition>> {
    for step in &definition.steps {
        let until = match &step.spec {
            StepSpec::Loop(spec) => Some((StepField::Until, &spec.until)),
            _ => None,
        };
        let when = step.spec.when().map(|when| (StepField::When, when));
        for (field, condition) in when.into_iter().chain(until) {
            check_condition(definition, agents, field, condition).map_err(|problem| {
                InvalidDefinition::new(
                    DefinitionFile::Workflow(definition.name.clone()),
                    Some(&step.key),
                    problem,
                )
            })?;
        }
    }
    Ok(())
}

fn check_condition(
    definition: &Definition,
    agents: &Agents,
    field: StepField,
    condition: &Condition,
) -> Result<(), Problem> {
    let declared = definition
        .step(&condition.step)
        .map_or_else(Vec::new, |step| declared(agents, &step.spec));
    if declared.contains(&condition.outcome) {
        return Ok(());
    }
    Err(Problem::UnknownOutcome {
        field,
        step: condition.step.clone(),
        outcome: condition.outcome.clone(),
        declared,
    })
}

fn declared(agents: &Agents, spec: &StepSpec) -> Vec<String> {
    let names: &[&str] = match spec {
        StepSpec::Agent(spec) => {
            return agents
                .get(&spec.agent)
                .map_or_else(Vec::new, |contract| contract.outcomes());
        }
        StepSpec::Approval(_) => &APPROVAL_OUTCOMES,
        StepSpec::Operation(spec) => Operation::find(&spec.run).map_or(&[], Operation::outcomes),
        StepSpec::Loop(_) | StepSpec::PerItem(_) => &[ENGINE_OUTCOME],
    };
    names.iter().map(|name| (*name).to_owned()).collect()
}
