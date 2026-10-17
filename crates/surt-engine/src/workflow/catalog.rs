use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    workflows: BTreeMap<String, &'static str>,
    agents: BTreeMap<String, AgentAssets>,
}

#[derive(Clone, Copy, Debug)]
pub struct AgentAssets {
    pub toml: &'static str,
    pub instructions: &'static str,
    pub schemas: &'static [(&'static str, &'static str)],
}

impl Catalog {
    pub fn new() -> Catalog {
        Catalog::default()
    }

    pub fn workflow(mut self, name: &str, toml: &'static str) -> Catalog {
        self.workflows.insert(name.to_owned(), toml);
        self
    }

    pub fn agent(mut self, id: &str, assets: AgentAssets) -> Catalog {
        self.agents.insert(id.to_owned(), assets);
        self
    }

    pub(crate) fn workflow_text(&self, name: &str) -> Option<&'static str> {
        self.workflows.get(name).copied()
    }

    pub(crate) fn agent_assets(&self, id: &str) -> Option<&AgentAssets> {
        self.agents.get(id)
    }
}

impl AgentAssets {
    pub(crate) fn schema(&self, path: &str) -> Option<&'static str> {
        self.schemas
            .iter()
            .find(|(candidate, _)| *candidate == path)
            .map(|(_, text)| *text)
    }
}
