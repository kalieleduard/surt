use surt_engine::project::{DefinitionFile, InvalidDefinition, Problem, Refusal};

use super::text;

const HUGE: &str =
    "unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`";

fn invalid(file: DefinitionFile, field: Option<&str>, problem: Problem) -> Refusal {
    Refusal::InvalidDefinition(Box::new(InvalidDefinition {
        file,
        field: field.map(str::to_owned),
        step: None,
        problem,
    }))
}

fn builder_file(path: &str) -> DefinitionFile {
    DefinitionFile::AgentFile {
        agent: "builder".to_owned(),
        path: path.to_owned(),
    }
}

pub(super) fn invalid_agent_file(field: Option<&str>) -> Refusal {
    invalid(
        DefinitionFile::Agent("builder".to_owned()),
        field,
        Problem::Syntax {
            line: Some(12),
            detail: HUGE.to_owned(),
        },
    )
}

#[test]
fn an_invalid_agent_toml_names_the_file_the_field_or_the_line_and_the_reason() {
    let texts = [
        text(&invalid_agent_file(Some("profile.effort"))),
        text(&invalid_agent_file(None)),
    ];

    assert_eq!(
        texts,
        [
            format!(
                "✖ .surt/agents/builder/agent.toml: profile.effort: {HUGE}\n  fix: edit .surt/agents/builder/agent.toml\n"
            ),
            format!(
                "✖ .surt/agents/builder/agent.toml:12: {HUGE}\n  fix: edit .surt/agents/builder/agent.toml\n"
            ),
        ]
    );
}

#[test]
fn a_stray_file_in_an_agent_directory_says_to_rename_or_remove_it() {
    let refusal = invalid(builder_file("notes.txt"), None, Problem::StrayAgentFile);

    assert_eq!(
        text(&refusal),
        "✖ .surt/agents/builder/notes.txt: surt reads only agent.toml, instructions.md and schemas/ here\n  \
         fix: rename or remove .surt/agents/builder/notes.txt\n"
    );
}

#[test]
fn a_missing_required_file_says_to_create_it() {
    let refusal = invalid(builder_file("instructions.md"), None, Problem::MissingFile);

    assert_eq!(
        text(&refusal),
        "✖ .surt/agents/builder/instructions.md: the file does not exist\n  \
         fix: create .surt/agents/builder/instructions.md\n"
    );
}

#[test]
fn an_unknown_workflow_lists_the_workflows_of_the_project_or_suggests_the_template() {
    let unknown = |known: &[&str]| {
        text(&Refusal::UnknownWorkflow {
            name: "feature".to_owned(),
            known: known.iter().map(|name| (*name).to_owned()).collect(),
        })
    };

    assert_eq!(
        [unknown(&["bugfix", "docs"]), unknown(&[])],
        [
            "✖ no workflow is named \"feature\" in .surt/workflows/\n  workflows: bugfix, docs\n  \
             fix: add .surt/workflows/feature.toml\n",
            "✖ no workflow is named \"feature\" in .surt/workflows/\n  .surt/workflows/ holds no workflow\n  \
             fix: surt init --template feature\n",
        ]
    );
}
