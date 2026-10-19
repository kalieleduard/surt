use std::fs;
use std::path::Path;

#[derive(Clone, Default)]
pub(in crate::project::tests) struct Catalog {
    workflows: Vec<(String, &'static str)>,
    agents: Vec<(String, AgentAssets)>,
}

#[derive(Clone, Copy)]
pub(in crate::project::tests) struct AgentAssets {
    pub(in crate::project::tests) toml: &'static str,
    pub(in crate::project::tests) instructions: &'static str,
    pub(in crate::project::tests) schemas: &'static [(&'static str, &'static str)],
}

impl Catalog {
    pub(in crate::project::tests) fn new() -> Catalog {
        Catalog::default()
    }

    pub(in crate::project::tests) fn workflow(mut self, name: &str, toml: &'static str) -> Catalog {
        self.workflows.push((name.to_owned(), toml));
        self
    }

    pub(in crate::project::tests) fn agent(mut self, id: &str, assets: AgentAssets) -> Catalog {
        self.agents.push((id.to_owned(), assets));
        self
    }

    pub(in crate::project::tests) fn write(&self, dot_surt: &Path) {
        for (name, toml) in &self.workflows {
            write(
                &dot_surt.join("workflows").join(format!("{name}.toml")),
                toml,
            );
        }
        for (id, assets) in &self.agents {
            let dir = dot_surt.join("agents").join(id);
            write(&dir.join("agent.toml"), assets.toml);
            write(&dir.join("instructions.md"), assets.instructions);
            for (path, text) in assets.schemas {
                write(&dir.join(path), text);
            }
        }
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the definition directory");
    fs::write(path, text).expect("the definition file");
}

const DRAFT_WORKFLOW: &str = r#"
schema = 1
name = "draft"
description = "Write and approve a draft."

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.write]
kind = "agent"
agent = "writer"

[steps.approve]
kind = "approval"
artifact = "draft"
rejection_artifact = "draft-rejection"
needs = ["write"]

[steps.drafting]
kind = "loop"
body = ["write", "approve"]
until = { step = "approve", outcome = "approved" }
needs = ["prepare"]
"#;

pub(in crate::project::tests) const WRITER_AGENT: &str = r#"
schema = 1
id = "writer"

[inputs.request]
format = "text"

[inputs.draft]
format = "markdown"
optional = true

[inputs.draft-rejection]
format = "text"
optional = true

[outputs.draft]
format = "markdown"

[outputs.notes]
format = "json"
json_schema = "schemas/notes.schema.json"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const WRITER_INSTRUCTIONS: &str = "Write the draft the request asks for.\n";

const NOTES_SCHEMA: &str = r#"{"type": "object"}"#;

pub(in crate::project::tests) const WRITER_SCHEMAS: &[(&str, &str)] =
    &[("schemas/notes.schema.json", NOTES_SCHEMA)];

pub(in crate::project::tests) fn writer() -> AgentAssets {
    AgentAssets {
        toml: WRITER_AGENT,
        instructions: WRITER_INSTRUCTIONS,
        schemas: WRITER_SCHEMAS,
    }
}

pub(in crate::project::tests) fn agent(toml: &'static str) -> AgentAssets {
    AgentAssets {
        toml,
        instructions: "Do the work.\n",
        schemas: &[],
    }
}

pub(in crate::project::tests) fn draft_catalog() -> Catalog {
    Catalog::new()
        .workflow("draft", DRAFT_WORKFLOW)
        .agent("writer", writer())
}
