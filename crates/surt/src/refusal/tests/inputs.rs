use surt_engine::project::{
    DeclaredInput, DefinitionFile, InputFormat, InputProblem, InvalidDefinition, Problem, Refusal,
};

use super::text;

fn declared() -> Vec<DeclaredInput> {
    vec![
        DeclaredInput {
            name: "base".to_owned(),
            format: InputFormat::GitRef,
            optional: true,
            default: Some("HEAD".to_owned()),
            description: None,
        },
        DeclaredInput {
            name: "settings".to_owned(),
            format: InputFormat::Json,
            optional: true,
            default: None,
            description: Some("tuning knobs".to_owned()),
        },
    ]
}

fn invalid_input(problem: InputProblem, declared: Vec<DeclaredInput>) -> Refusal {
    Refusal::InvalidInput {
        workflow: "deploy".to_owned(),
        problem,
        declared,
    }
}

#[test]
fn an_input_that_is_not_json_names_the_parser_message_and_lists_the_inputs() {
    let refusal = invalid_input(
        InputProblem::NotJson {
            name: "settings".to_owned(),
            detail: "expected value at line 1 column 1".to_owned(),
        },
        declared(),
    );

    assert_eq!(
        text(&refusal),
        "✖ input \"settings\" is not JSON: expected value at line 1 column 1\n  \
         inputs of deploy:\n    \
         base      git_ref  optional  (default HEAD)\n    \
         settings  json     optional  tuning knobs\n  \
         fix: pass JSON as --input settings=<value>\n"
    );
}

#[test]
fn a_duplicate_or_empty_input_says_how_to_pass_it() {
    let duplicate = invalid_input(
        InputProblem::Duplicate {
            name: "base".to_owned(),
        },
        declared(),
    );
    let empty = invalid_input(
        InputProblem::Empty {
            name: "settings".to_owned(),
        },
        Vec::new(),
    );

    assert!(
        text(&duplicate).ends_with("  fix: pass --input base once\n"),
        "{}",
        text(&duplicate)
    );
    assert_eq!(
        text(&empty),
        "✖ input \"settings\" is empty\n  workflow deploy takes no inputs\n  \
         fix: pass --input settings=<value>\n"
    );
}

#[test]
fn input_declaration_problems_name_the_field_of_the_workflow_file() {
    let refusal = Refusal::InvalidDefinition(Box::new(InvalidDefinition {
        file: DefinitionFile::Workflow("deploy".to_owned()),
        field: Some("subject.from".to_owned()),
        step: None,
        problem: Problem::SubjectInputNotText {
            input: "settings".to_owned(),
            format: InputFormat::Json,
        },
    }));

    assert_eq!(
        text(&refusal),
        "✖ .surt/workflows/deploy.toml: subject.from: the subject comes from \"settings\", a json input; it must be text or markdown\n  \
         fix: edit .surt/workflows/deploy.toml\n"
    );
}
