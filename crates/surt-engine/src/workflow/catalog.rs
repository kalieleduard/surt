use std::collections::BTreeMap;

use super::closure;
use super::error::InvalidDefinition;
use super::flow::Available;
use super::sources::{AgentSources, LocalAssets};

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    workflows: BTreeMap<String, &'static str>,
    agents: BTreeMap<String, AgentAssets>,
    effective_tomls: BTreeMap<String, String>,
    local_assets: BTreeMap<String, LocalAssets>,
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

    pub(crate) fn agent_assets(&self, id: &str) -> Option<&AgentAssets> {
        self.agents.get(id)
    }

    pub(crate) fn agent_sources(&self, id: &str) -> Option<AgentSources<'_>> {
        self.agents.get(id).map(|assets| AgentSources {
            assets,
            local: self.local_assets.get(id),
        })
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

    pub(super) fn with_local_assets(mut self, id: &str, local: LocalAssets) -> Catalog {
        self.local_assets.insert(id.to_owned(), local);
        self
    }

    pub(super) fn with_override_from(mut self, effective: &Catalog, id: &str) -> Catalog {
        if let Some(toml) = effective.effective_tomls.get(id) {
            self.effective_tomls.insert(id.to_owned(), toml.clone());
        }
        if let Some(local) = effective.local_assets.get(id) {
            self.local_assets.insert(id.to_owned(), local.clone());
        }
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
