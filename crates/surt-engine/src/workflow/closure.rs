use super::agent::AgentContract;
use super::catalog::Catalog;
use super::definition::Definition;
use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::flow::{self, Agents, Available};
use super::steps::StepSpec;
use super::{outcomes, structure};

pub(super) struct Closure {
    pub(super) root: Definition,
    pub(super) children: Vec<Definition>,
    pub(super) agents: Agents,
}

struct Referrer<'a> {
    workflow: &'a str,
    step: &'a str,
}

struct Resolver<'a> {
    catalog: &'a Catalog,
    children: Vec<Definition>,
    agents: Agents,
    stack: Vec<String>,
}

pub(super) fn resolve(
    catalog: &Catalog,
    root: &str,
    external: &Available,
) -> Result<Closure, Box<InvalidDefinition>> {
    let mut resolver = Resolver {
        catalog,
        children: Vec::new(),
        agents: Agents::new(),
        stack: Vec::new(),
    };
    let root = resolver.visit(root, None, external)?;
    Ok(Closure {
        root,
        children: resolver.children,
        agents: resolver.agents,
    })
}

impl Resolver<'_> {
    fn visit(
        &mut self,
        name: &str,
        referrer: Option<Referrer>,
        external: &Available,
    ) -> Result<Definition, Box<InvalidDefinition>> {
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
            }));
        }
        let text = self.catalog.workflow_text(name).ok_or_else(|| {
            refuse(Problem::UnknownWorkflow {
                name: name.to_owned(),
            })
        })?;
        let definition = Definition::parse(name, text)?;
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

    fn load_agents(&mut self, definition: &Definition) -> Result<(), Box<InvalidDefinition>> {
        for step in &definition.steps {
            let StepSpec::Agent(spec) = &step.spec else {
                continue;
            };
            if self.agents.contains_key(&spec.agent) {
                continue;
            }
            let sources = self.catalog.agent_sources(&spec.agent).ok_or_else(|| {
                InvalidDefinition::new(
                    DefinitionFile::Workflow(definition.name.clone()),
                    Some(&step.key),
                    Problem::UnknownAgent {
                        name: spec.agent.clone(),
                    },
                )
            })?;
            let toml = self
                .catalog
                .agent_toml(&spec.agent)
                .unwrap_or(sources.assets.toml);
            let contract = AgentContract::parse(&spec.agent, toml, &sources)?;
            self.agents.insert(spec.agent.clone(), contract);
        }
        Ok(())
    }
}
