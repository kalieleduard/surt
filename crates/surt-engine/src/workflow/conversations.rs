use super::agent::ConversationScope;
use super::definition::Definition;
use super::error::{DefinitionFile, InvalidDefinition, Problem};
use super::flow::Agents;
use super::steps::StepSpec;

pub(super) fn check(
    definition: &Definition,
    agents: &Agents,
) -> Result<(), Box<InvalidDefinition>> {
    for step in &definition.steps {
        let StepSpec::Agent(spec) = &step.spec else {
            continue;
        };
        let continues = agents
            .get(&spec.agent)
            .is_some_and(|agent| agent.profile.conversation == ConversationScope::Loop);
        if continues && definition.enclosing_loop(&step.key).is_none() {
            return Err(InvalidDefinition::new(
                DefinitionFile::Workflow(definition.name.clone()),
                Some(&step.key),
                Problem::ConversationOutsideLoop {
                    agent: spec.agent.clone(),
                },
            ));
        }
    }
    Ok(())
}
