use surt_engine::project::{DefinitionFile, InvalidDefinition, Problem, Refusal, StepField};

use super::text;
use crate::symbols::Symbols;

fn problem_line(problem: Problem) -> String {
    let refusal = Refusal::InvalidDefinition(Box::new(InvalidDefinition {
        file: DefinitionFile::Workflow("feature".to_owned()),
        step: None,
        problem,
    }));
    text(&refusal)
        .lines()
        .nth(1)
        .expect("a context line")
        .trim()
        .to_owned()
}

fn owned(text: &str) -> String {
    text.to_owned()
}

#[test]
fn every_definition_problem_is_described_in_one_line_of_words() {
    let cases = [
        (
            Problem::Syntax {
                line: Some(3),
                detail: owned("expected `=`"),
            },
            "line 3: expected `=`",
        ),
        (
            Problem::Syntax {
                line: None,
                detail: owned("bad document"),
            },
            "bad document",
        ),
        (
            Problem::InvalidStep {
                detail: owned("missing field `agent`"),
            },
            "missing field `agent`",
        ),
        (
            Problem::UnsupportedSchema { found: 2 },
            "schema = 2 is not supported; this surt reads schema = 1",
        ),
        (
            Problem::NameMismatch {
                found: owned("other"),
            },
            "it is named \"other\" inside",
        ),
        (Problem::MissingKind, "the step has no kind"),
        (
            Problem::UnknownKind {
                kind: owned("script"),
            },
            "unknown step kind \"script\"",
        ),
        (
            Problem::UnknownWorkflow { name: owned("x") },
            "no workflow is named \"x\"",
        ),
        (
            Problem::RecursiveWorkflow { name: owned("x") },
            "workflow \"x\" runs itself through a per_item step",
        ),
        (
            Problem::UnknownAgent { name: owned("x") },
            "no agent is named \"x\"",
        ),
        (
            Problem::UnknownOperation { name: owned("x") },
            "no operation is named \"x\"",
        ),
        (
            Problem::UnknownStep {
                field: StepField::Needs,
                name: owned("x"),
            },
            "needs names an unknown step \"x\"",
        ),
        (
            Problem::UnknownStep {
                field: StepField::Body,
                name: owned("x"),
            },
            "body names an unknown step \"x\"",
        ),
        (
            Problem::OutOfScope {
                field: StepField::When,
                name: owned("x"),
            },
            "when names \"x\" from another scope",
        ),
        (
            Problem::OutOfScope {
                field: StepField::Until,
                name: owned("x"),
            },
            "until names \"x\" from another scope",
        ),
        (
            Problem::SharedBodyStep { name: owned("x") },
            "\"x\" is in the body of two loops",
        ),
        (
            Problem::NestedLoop { name: owned("x") },
            "loop \"x\" is inside another loop",
        ),
        (
            Problem::Cycle {
                path: vec![owned("a"), owned("b"), owned("a")],
            },
            "the steps form a cycle: a → b → a",
        ),
        (
            Problem::UntilOutsideBody { name: owned("x") },
            "until names \"x\", which is not in the loop's body",
        ),
        (
            Problem::UnknownAgentInput { input: owned("x") },
            "maps input \"x\", which the agent does not declare",
        ),
        (
            Problem::MissingInput { input: owned("x") },
            "needs \"x\", which nothing before it produces",
        ),
        (
            Problem::UnproducedArtifact {
                artifact: owned("x"),
            },
            "\"x\" is not produced before this step",
        ),
        (
            Problem::NotAnItemList {
                artifact: owned("x"),
            },
            "\"x\" is not an item list",
        ),
        (
            Problem::DuplicateRejectionArtifact {
                artifact: owned("x"),
            },
            "rejection artifact \"x\" is declared twice",
        ),
        (
            Problem::MissingAsset {
                path: owned("schemas/x.json"),
            },
            "\"schemas/x.json\" is not bundled with the agent",
        ),
        (
            Problem::MisplacedJsonSchema { name: owned("x") },
            "\"x\" has a json_schema but is neither json nor item_list",
        ),
        (
            Problem::MisplacedItemFiles { name: owned("x") },
            "\"x\" declares item_files but is not an item_list",
        ),
        (
            Problem::UnsupportedItemFileFormat { name: owned("x") },
            "item file \"x\" has an unsupported format",
        ),
        (
            Problem::MissingClaudeProfile,
            "runtime claude-code needs a [profile.claude] table",
        ),
        (
            Problem::MisplacedApiKeyEnv,
            "api_key_env needs auth = \"api_key\"",
        ),
        (
            Problem::InvalidBudget,
            "max_budget_usd must be a positive number",
        ),
    ];

    for (problem, expected) in cases {
        assert_eq!(problem_line(problem), expected);
    }
}

#[test]
fn a_cycle_in_ascii_mode_is_drawn_with_ascii_arrows() {
    let line = problem_line(Problem::Cycle {
        path: vec![owned("a"), owned("b"), owned("a")],
    });

    assert_eq!(
        Symbols::Ascii.render(&line),
        "the steps form a cycle: a -> b -> a"
    );
}
