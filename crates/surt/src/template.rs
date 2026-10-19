mod feature;

pub(crate) const FEATURE_TEMPLATE: &str = "feature";
pub(crate) const TEMPLATES: [&str; 1] = [FEATURE_TEMPLATE];

const WORKFLOWS_DIR: &str = "workflows";
const AGENTS_DIR: &str = "agents";

type AgentFiles = &'static [(&'static str, &'static str)];

pub(crate) struct Template {
    workflows: &'static [(&'static str, &'static str)],
    agents: &'static [(&'static str, AgentFiles)],
    next: &'static str,
}

pub(crate) fn named(name: &str) -> Option<Template> {
    match name {
        FEATURE_TEMPLATE => Some(feature::TEMPLATE),
        _ => None,
    }
}

impl Template {
    pub(crate) fn next_command(&self) -> &'static str {
        self.next
    }

    pub(crate) fn files(&self) -> Vec<(String, &'static str)> {
        let workflows = self
            .workflows
            .iter()
            .map(|(file, text)| (format!("{WORKFLOWS_DIR}/{file}"), *text));
        let agents = self.agents.iter().flat_map(|(id, files)| {
            files
                .iter()
                .map(move |(path, text)| (format!("{AGENTS_DIR}/{id}/{path}"), *text))
        });
        workflows.chain(agents).collect()
    }
}

#[cfg(test)]
mod tests;
