use crate::project::{AgentAssets, Catalog};

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
