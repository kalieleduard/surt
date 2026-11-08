use std::path::PathBuf;

use super::{Place, resolve};

fn surt() -> PathBuf {
    PathBuf::from("project").join(".surt")
}

#[test]
fn a_workflow_file_resolves_inside_the_workflows_folder() {
    let (path, place) = resolve(&surt(), "workflows/feature.toml").expect("resolves");

    assert_eq!(path, surt().join("workflows").join("feature.toml"));
    assert_eq!(place, Place::Workflow);
}

#[test]
fn the_agent_toml_and_instructions_are_core_agent_files() {
    let (instructions, first) = resolve(&surt(), "agents/spec/instructions.md").expect("resolves");
    let (_, second) = resolve(&surt(), "agents/spec/agent.toml").expect("resolves");

    assert_eq!(
        instructions,
        surt().join("agents").join("spec").join("instructions.md")
    );
    assert_eq!((first, second), (Place::AgentCore, Place::AgentCore));
}

#[test]
fn references_and_schemas_are_extra_agent_files() {
    let (_, reference) =
        resolve(&surt(), "agents/spec/references/spec-template.md").expect("resolves");
    let (_, schema) =
        resolve(&surt(), "agents/tasks/schemas/nested/manifest.json").expect("resolves");

    assert_eq!((reference, schema), (Place::AgentExtra, Place::AgentExtra));
}

#[test]
fn paths_that_leave_the_definitions_are_refused() {
    let refused = [
        "../config.toml",
        "agents/../../secret",
        "agents/spec/../../config.toml",
        "config.toml",
        "workflows/.toml",
        "workflows/feature.txt",
        "agents/spec/notes.md",
        "agents/spec/references",
        "agents\\spec\\agent.toml",
        "C:/agents/spec/agent.toml",
        "/agents/spec/agent.toml",
        "",
    ];

    for path in refused {
        assert!(resolve(&surt(), path).is_err(), "{path} should be refused");
    }
}
