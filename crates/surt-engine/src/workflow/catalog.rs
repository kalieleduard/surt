use std::collections::BTreeMap;

use super::closure;
use super::error::InvalidDefinition;
use super::flow::Available;

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    workflows: BTreeMap<String, &'static str>,
    agents: BTreeMap<String, AgentAssets>,
    effective_tomls: BTreeMap<String, String>,
    checks: BTreeMap<String, Vec<(String, DeliveryCheck)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryCheck {
    CommitEvidence,
    ReviewedCommit,
    NoBlockingFinding { outcome: &'static str },
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

    pub fn delivery_check(mut self, agent: &str, output: &str, check: DeliveryCheck) -> Catalog {
        self.checks
            .entry(agent.to_owned())
            .or_default()
            .push((output.to_owned(), check));
        self
    }

    pub fn check(
        &self,
        workflow: &str,
        initial_artifacts: &[&str],
    ) -> Result<(), Box<InvalidDefinition>> {
        let external: Available = initial_artifacts
            .iter()
            .map(|name| ((*name).to_owned(), None))
            .collect();
        closure::resolve(self, workflow, &external).map(|_| ())
    }

    pub(crate) fn workflow_text(&self, name: &str) -> Option<&'static str> {
        self.workflows.get(name).copied()
    }

    pub fn delivery_checks(&self, agent: &str) -> &[(String, DeliveryCheck)] {
        self.checks.get(agent).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn agent_assets(&self, id: &str) -> Option<&AgentAssets> {
        self.agents.get(id)
    }

    pub(crate) fn agent_ids(&self) -> impl Iterator<Item = &str> {
        self.agents.keys().map(String::as_str)
    }

    pub(crate) fn agent_toml(&self, id: &str) -> Option<&str> {
        self.effective_tomls
            .get(id)
            .map(String::as_str)
            .or_else(|| self.agents.get(id).map(|assets| assets.toml))
    }

    pub(super) fn with_effective_toml(mut self, id: &str, toml: String) -> Catalog {
        self.effective_tomls.insert(id.to_owned(), toml);
        self
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
