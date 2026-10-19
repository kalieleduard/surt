use std::collections::BTreeMap;

use super::agent::AgentContract;
use super::agent_schemas;
use super::definition::Definition;
use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::files::{AgentFiles, DefinitionSource};
use super::flow::{self, Agents, Available};
use super::freeze::FreezeError;
use super::steps::StepSpec;
use super::{outcomes, structure};

pub(super) struct Closure {
    pub(super) root: Definition,
    pub(super) children: Vec<Definition>,
    pub(super) agents: Agents,
    pub(super) files: BTreeMap<String, AgentFiles>,
}

struct Referrer<'a> {
    workflow: &'a str,
    step: &'a str,
}

struct Resolver<'a, S> {
    source: &'a S,
    children: Vec<Definition>,
    agents: Agents,
    files: BTreeMap<String, AgentFiles>,
    stack: Vec<String>,
}

pub(crate) fn validate<'a>(
    source: &impl DefinitionSource,
    root: &str,
    initial_artifacts: impl IntoIterator<Item = &'a str>,
) -> Result<(), FreezeError> {
    resolve(source, root, initial_artifacts).map(|_| ())
}

pub(super) fn resolve<'a>(
    source: &impl DefinitionSource,
    root: &str,
    initial_artifacts: impl IntoIterator<Item = &'a str>,
) -> Result<Closure, FreezeError> {
    let external: Available = initial_artifacts
        .into_iter()
        .map(|name| (name.to_owned(), None))
        .collect();
    let mut resolver = Resolver {
        source,
        children: Vec::new(),
        agents: Agents::new(),
        files: BTreeMap::new(),
        stack: Vec::new(),
    };
    let root = resolver.visit(root, None, &external)?;
    Ok(Closure {
        root,
        children: resolver.children,
        agents: resolver.agents,
        files: resolver.files,
    })
}

impl<S: DefinitionSource> Resolver<'_, S> {
    fn visit(
        &mut self,
        name: &str,
        referrer: Option<Referrer>,
        external: &Available,
    ) -> Result<Definition, FreezeError> {
        let definition = self.definition(name, referrer)?;
        structure::check(&definition)?;
        self.load_agents(&definition)?;
        outcomes::check(&definition, &self.agents)?;
        let uses = flow::check(&definition, &self.agents, external)?;
        self.stack.push(name.to_owned());
        for child in uses {
            let referrer = Referrer {
                workflow: name,
                step: &child.step,
            };
            let visited = self.visit(&child.workflow, Some(referrer), &child.external)?;
            if self.children.iter().all(|known| known.name != visited.name) {
                self.children.push(visited);
            }
        }
        self.stack.pop();
        Ok(definition)
    }

    fn definition(
        &self,
        name: &str,
        referrer: Option<Referrer>,
    ) -> Result<Definition, FreezeError> {
        let refuse = |problem| match &referrer {
            Some(referrer) => InvalidDefinition::new(
                DefinitionFile::Workflow(referrer.workflow.to_owned()),
                Some(referrer.step),
                problem,
            ),
            None => {
                InvalidDefinition::new(DefinitionFile::Workflow(name.to_owned()), None, problem)
            }
        };
        if self.stack.iter().any(|visiting| visiting == name) {
            return Err(refuse(Problem::RecursiveWorkflow {
                name: name.to_owned(),
            })
            .into());
        }
        let text = self.source.workflow(name)?.ok_or_else(|| {
            refuse(Problem::UnknownWorkflow {
                name: name.to_owned(),
            })
        })?;
        Ok(Definition::parse(name, &text)?)
    }

    fn load_agents(&mut self, definition: &Definition) -> Result<(), FreezeError> {
        for step in &definition.steps {
            let StepSpec::Agent(spec) = &step.spec else {
                continue;
            };
            if self.agents.contains_key(&spec.agent) {
                continue;
            }
            let files = self.source.agent(&spec.agent)?.ok_or_else(|| {
                InvalidDefinition::new(
                    DefinitionFile::Workflow(definition.name.clone()),
                    Some(&step.key),
                    Problem::UnknownAgent {
                        name: spec.agent.clone(),
                    },
                )
            })?;
            agent_schemas::check(&spec.agent, &files)?;
            let contract = AgentContract::parse(&spec.agent, &files.toml, &files)?;
            self.agents.insert(spec.agent.clone(), contract);
            self.files.insert(spec.agent.clone(), files);
        }
        Ok(())
    }
}
