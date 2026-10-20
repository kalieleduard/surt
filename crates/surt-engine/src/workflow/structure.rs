use std::collections::{BTreeMap, BTreeSet};

use super::definition::{Definition, Step};
use super::error::{DefinitionFile, InvalidDefinition, Problem, StepField};
use super::steps::StepSpec;
use crate::operations::Operation;

pub(super) fn check(definition: &Definition) -> Result<(), Box<InvalidDefinition>> {
    let invalid = |step: &str, problem| {
        InvalidDefinition::new(
            DefinitionFile::Workflow(definition.name.clone()),
            Some(step),
            problem,
        )
    };
    let loops = loop_membership(definition).map_err(|(step, problem)| invalid(step, problem))?;
    for step in &definition.steps {
        check_step(definition, &loops, step).map_err(|problem| invalid(&step.key, problem))?;
    }
    check_human_artifacts(definition).map_err(|(step, problem)| invalid(step, problem))?;
    check_acyclic(definition).map_err(|(step, problem)| invalid(step, problem))
}

fn loop_membership(definition: &Definition) -> Result<BTreeMap<&str, &str>, (&str, Problem)> {
    let mut loops = BTreeMap::new();
    for step in &definition.steps {
        let StepSpec::Loop(spec) = &step.spec else {
            continue;
        };
        for member in &spec.body {
            let fail = |problem| Err((step.key.as_str(), problem));
            match definition.step(member).map(|found| &found.spec) {
                None => fail(Problem::UnknownStep {
                    field: StepField::Body,
                    name: member.clone(),
                })?,
                Some(StepSpec::Loop(_)) => fail(Problem::NestedLoop {
                    name: member.clone(),
                })?,
                Some(_) if loops.insert(member.as_str(), step.key.as_str()).is_some() => {
                    fail(Problem::SharedBodyStep {
                        name: member.clone(),
                    })?
                }
                Some(_) => {}
            }
        }
    }
    Ok(loops)
}

fn check_step(
    definition: &Definition,
    loops: &BTreeMap<&str, &str>,
    step: &Step,
) -> Result<(), Problem> {
    let scope = loops.get(step.key.as_str());
    let references = step
        .spec
        .needs()
        .iter()
        .map(|name| (StepField::Needs, name))
        .chain(step.spec.when().map(|when| (StepField::When, &when.step)));
    for (field, name) in references {
        if definition.step(name).is_none() {
            return Err(Problem::UnknownStep {
                field,
                name: name.clone(),
            });
        }
        if loops.get(name.as_str()) != scope {
            return Err(Problem::OutOfScope {
                field,
                name: name.clone(),
            });
        }
    }
    match &step.spec {
        StepSpec::Loop(spec) if definition.step(&spec.until.step).is_none() => {
            Err(Problem::UnknownStep {
                field: StepField::Until,
                name: spec.until.step.clone(),
            })
        }
        StepSpec::Loop(spec) if !spec.body.contains(&spec.until.step) => {
            Err(Problem::UntilOutsideBody {
                name: spec.until.step.clone(),
            })
        }
        StepSpec::Operation(spec) if Operation::find(&spec.run).is_none() => {
            Err(Problem::UnknownOperation {
                name: spec.run.clone(),
            })
        }
        _ => Ok(()),
    }
}

fn check_human_artifacts(definition: &Definition) -> Result<(), (&str, Problem)> {
    let mut seen = BTreeSet::new();
    for step in &definition.steps {
        let Some((artifact, duplicate)) = human_artifact(&step.spec) else {
            continue;
        };
        if !seen.insert(artifact) {
            return Err((step.key.as_str(), duplicate(artifact.to_owned())));
        }
    }
    Ok(())
}

fn human_artifact(spec: &StepSpec) -> Option<(&str, fn(String) -> Problem)> {
    match spec {
        StepSpec::Approval(spec) => Some((&spec.rejection_artifact, |artifact| {
            Problem::DuplicateRejectionArtifact { artifact }
        })),
        StepSpec::Input(spec) => Some((&spec.answer_artifact, |artifact| {
            Problem::DuplicateAnswerArtifact { artifact }
        })),
        _ => None,
    }
}

fn check_acyclic(definition: &Definition) -> Result<(), (&str, Problem)> {
    let mut finished = BTreeSet::new();
    for step in &definition.steps {
        let mut path = Vec::new();
        if let Some(cycle) = find_cycle(definition, &step.key, &mut path, &mut finished) {
            return Err((step.key.as_str(), Problem::Cycle { path: cycle }));
        }
    }
    Ok(())
}

fn find_cycle<'a>(
    definition: &'a Definition,
    key: &'a str,
    path: &mut Vec<&'a str>,
    finished: &mut BTreeSet<&'a str>,
) -> Option<Vec<String>> {
    if let Some(start) = path.iter().position(|visited| *visited == key) {
        let mut cycle: Vec<String> = path[start..].iter().map(|key| (*key).to_owned()).collect();
        cycle.push(key.to_owned());
        return Some(cycle);
    }
    if finished.contains(key) {
        return None;
    }
    let step = definition.step(key)?;
    path.push(key);
    let cycle = dependencies(step).find_map(|next| find_cycle(definition, next, path, finished));
    path.pop();
    finished.insert(key);
    cycle
}

pub(super) fn dependencies(step: &Step) -> impl Iterator<Item = &str> {
    step.spec
        .needs()
        .iter()
        .map(String::as_str)
        .chain(step.spec.when().map(|when| when.step.as_str()))
}
