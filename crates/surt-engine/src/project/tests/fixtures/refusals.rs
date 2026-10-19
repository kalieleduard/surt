use std::fs;

use super::{ProjectDir, default_config, new_run, writer};
use crate::project::{
    Catalog, DefinitionFile, Error, InvalidDefinition, InvalidOverride, Problem, Refusal,
};

pub(in crate::project::tests) async fn refusal(catalog: Catalog) -> InvalidDefinition {
    let dir = ProjectDir::new();
    let project = dir.project(catalog, default_config()).await;

    let result = project.create_run(new_run("flow")).await;

    dir.assert_nothing_written().await;
    match result {
        Err(Error::Refused(Refusal::InvalidDefinition(invalid))) => *invalid,
        other => panic!("expected an invalid definition, got {other:?}"),
    }
}

pub(in crate::project::tests) async fn workflow_refusal(text: &'static str) -> InvalidDefinition {
    refusal(
        Catalog::new()
            .workflow("flow", text)
            .agent("writer", writer()),
    )
    .await
}

pub(in crate::project::tests) fn in_step(step: &str, problem: Problem) -> InvalidDefinition {
    InvalidDefinition {
        file: DefinitionFile::Workflow("flow".to_owned()),
        step: Some(step.to_owned()),
        problem,
    }
}

pub(in crate::project::tests) async fn override_refusal(
    dir: &ProjectDir,
    catalog: Catalog,
    workflow: &str,
) -> InvalidOverride {
    let project = dir.project(catalog, default_config()).await;

    let result = project.create_run(new_run(workflow)).await;

    dir.assert_nothing_written().await;
    match result {
        Err(Error::Refused(Refusal::InvalidAgentOverride(invalid))) => *invalid,
        other => panic!("expected an invalid override, got {other:?}"),
    }
}

impl ProjectDir {
    pub(in crate::project::tests) fn write_local(&self, agent: &str, rel_path: &str, text: &str) {
        let path = self.layout().agents().join(agent).join(rel_path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the agent directory");
        fs::write(path, text).expect("the local file");
    }

    pub(in crate::project::tests) async fn frozen_agent(&self, agent: &str) -> toml::Table {
        let text = self
            .frozen_agent_file(agent, "agent.toml")
            .await
            .expect("the agent.toml is frozen");
        toml::from_str(&text).expect("the frozen agent is TOML")
    }

    pub(in crate::project::tests) async fn frozen_agent_file(
        &self,
        agent: &str,
        rel_path: &str,
    ) -> Option<String> {
        let sql = format!(
            "SELECT b.bytes FROM snapshot s JOIN snapshot_file f ON f.snapshot_sha256 = s.sha256 JOIN blob b ON b.sha256 = f.blob_sha256 WHERE s.kind = 'agent' AND s.name = '{agent}' AND f.rel_path = '{rel_path}'"
        );
        let stored: Vec<(Vec<u8>,)> = self.rows(&sql).await;
        stored
            .into_iter()
            .next()
            .map(|(bytes,)| String::from_utf8(bytes).expect("the frozen file is UTF-8"))
    }
}
