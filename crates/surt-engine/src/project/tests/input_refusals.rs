use super::fixtures::{ProjectDir, input, new_run};
use super::inputs::inputs_project;
use crate::project::{DeclaredInput, Error, InputFormat, InputProblem, Refusal};

fn declared() -> Vec<DeclaredInput> {
    let declared =
        |name: &str, format, optional, default: Option<&str>, description: Option<&str>| {
            DeclaredInput {
                name: name.to_owned(),
                format,
                optional,
                default: default.map(str::to_owned),
                description: description.map(str::to_owned),
            }
        };
    vec![
        declared(
            "request",
            InputFormat::Text,
            false,
            None,
            Some("what to build"),
        ),
        declared("notes", InputFormat::Markdown, true, None, None),
        declared("settings", InputFormat::Json, true, None, None),
        declared(
            "base",
            InputFormat::GitRef,
            true,
            Some("HEAD"),
            Some("where to start"),
        ),
    ]
}

async fn refused(values: &[(&str, &str)]) -> InputProblem {
    let dir = ProjectDir::new();
    let project = inputs_project(&dir).await;
    let mut run = new_run("inputs");
    run.inputs = values
        .iter()
        .map(|(name, value)| input(name, value))
        .collect();

    let created = project.create_run(run).await;

    dir.assert_nothing_written().await;
    match created {
        Err(Error::Refused(Refusal::InvalidInput {
            workflow,
            problem,
            declared: listed,
        })) => {
            assert_eq!(workflow, "inputs");
            assert_eq!(listed, declared());
            problem
        }
        other => panic!("expected an input refusal, got {other:?}"),
    }
}

fn named(name: &str) -> String {
    name.to_owned()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_the_workflow_does_not_declare_is_refused() {
    let problem = refused(&[("request", "add login"), ("colour", "blue")]).await;

    assert_eq!(
        problem,
        InputProblem::Unknown {
            name: named("colour")
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_input_given_twice_is_refused() {
    let problem = refused(&[("request", "add login"), ("request", "again")]).await;

    assert_eq!(
        problem,
        InputProblem::Duplicate {
            name: named("request")
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_required_input_left_out_is_refused() {
    let problem = refused(&[("notes", "# Notes")]).await;

    assert_eq!(
        problem,
        InputProblem::Missing {
            name: named("request")
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_text_or_markdown_input_is_refused() {
    let text = refused(&[("request", "  \n")]).await;
    let markdown = refused(&[("request", "add login"), ("notes", "")]).await;

    assert_eq!(
        text,
        InputProblem::Empty {
            name: named("request")
        }
    );
    assert_eq!(
        markdown,
        InputProblem::Empty {
            name: named("notes")
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_json_input_that_does_not_parse_is_refused_with_the_parser_message() {
    let problem = refused(&[("request", "add login"), ("settings", "{retries")]).await;

    assert!(
        matches!(&problem, InputProblem::NotJson { name, detail } if name == "settings" && !detail.is_empty()),
        "{problem:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_git_ref_that_names_no_commit_is_refused_with_the_reference() {
    let problem = refused(&[("request", "add login"), ("base", "no-such-branch")]).await;

    assert_eq!(
        problem,
        InputProblem::Unresolved {
            name: named("base"),
            reference: named("no-such-branch"),
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_subject_input_check_refuses_the_same_problems_before_any_run_exists() {
    let dir = ProjectDir::new();
    let project = inputs_project(&dir).await;

    let given = [input("colour", "blue")];

    let checked = project.subject_input("inputs", &given);

    assert!(
        matches!(
            checked,
            Err(Error::Refused(Refusal::InvalidInput {
                problem: InputProblem::Unknown { .. },
                ..
            }))
        ),
        "{checked:?}"
    );
    dir.assert_nothing_written().await;
}
