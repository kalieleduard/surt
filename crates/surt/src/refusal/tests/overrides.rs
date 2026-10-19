use surt_engine::project::{InvalidOverride, Problem, Refusal, StepField, StepUse, WorkflowStep};

use super::text;

pub(super) fn invalid_override(field: Option<&str>) -> Refusal {
    Refusal::InvalidAgentOverride(Box::new(InvalidOverride {
        file: ".surt/agents/builder/agent.toml".to_owned(),
        field: field.map(str::to_owned),
        needed_by: None,
        problem: Problem::Syntax {
            line: None,
            detail:
                "unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`"
                    .to_owned(),
        },
    }))
}

#[test]
fn an_invalid_agent_override_names_the_file_the_field_and_the_reason() {
    let texts = [
        text(&invalid_override(Some("profile.effort"))),
        text(&invalid_override(None)),
    ];

    assert_eq!(
        texts,
        [
            "✖ .surt/agents/builder/agent.toml: profile.effort: unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`\n  \
             fix: edit .surt/agents/builder/agent.toml\n",
            "✖ .surt/agents/builder/agent.toml: unknown variant `huge`, expected one of `low`, `medium`, `high`, `xhigh`, `max`\n  \
             fix: edit .surt/agents/builder/agent.toml\n",
        ]
    );
}

#[test]
fn an_override_that_breaks_a_workflow_names_the_step_that_needs_it() {
    let refusal = Refusal::InvalidAgentOverride(Box::new(InvalidOverride {
        file: ".surt/agents/reviewer/agent.toml".to_owned(),
        field: None,
        needed_by: Some(WorkflowStep {
            workflow: "task-cycle".to_owned(),
            step: "review-loop".to_owned(),
            field: Some(StepUse::Until),
        }),
        problem: Problem::UnknownOutcome {
            field: StepField::Until,
            step: "review".to_owned(),
            outcome: "changes_requested".to_owned(),
            declared: vec!["approved".to_owned()],
        },
    }));

    assert_eq!(
        text(&refusal),
        "✖ .surt/agents/reviewer/agent.toml: outcomes no longer include \"changes_requested\"\n  \
         needed by: task-cycle step review-loop (until)\n  \
         fix: edit .surt/agents/reviewer/agent.toml\n"
    );
}

#[test]
fn a_stray_file_under_the_agents_directory_says_to_rename_or_remove_it() {
    let refusal = Refusal::InvalidAgentOverride(Box::new(InvalidOverride {
        file: ".surt/agents/builder/notes.txt".to_owned(),
        field: None,
        needed_by: None,
        problem: Problem::StrayAgentFile,
    }));

    assert_eq!(
        text(&refusal),
        "✖ .surt/agents/builder/notes.txt: surt reads only agent.toml, instructions.md and schemas/ here\n  \
         fix: rename or remove .surt/agents/builder/notes.txt\n"
    );
}
