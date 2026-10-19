use surt_engine::project::{DeclaredInput, InputProblem};

use crate::message::Message;
use crate::retry::quoted;
use crate::table;

const ROW_INDENT: &str = "  ";
const HEAD: &str = "HEAD";
const WORKFLOWS_DIR: &str = ".surt/workflows";

pub(super) fn invalid(
    workflow: &str,
    problem: &InputProblem,
    declared: &[DeclaredInput],
) -> Message {
    listed(
        Message::new(headline(workflow, problem)),
        workflow,
        declared,
    )
    .way("fix", fix(workflow, problem))
}

fn headline(workflow: &str, problem: &InputProblem) -> String {
    match problem {
        InputProblem::Unknown { name } => {
            format!("workflow {workflow} has no input {}", quoted(name))
        }
        InputProblem::Duplicate { name } => format!("input {} is given twice", quoted(name)),
        InputProblem::Missing { name } => format!("input {} is required", quoted(name)),
        InputProblem::Empty { name } => format!("input {} is empty", quoted(name)),
        InputProblem::NotJson { name, detail } => {
            format!("input {} is not JSON: {detail}", quoted(name))
        }
        InputProblem::Unresolved { name, reference } if reference == HEAD => format!(
            "input {}: HEAD does not name a commit; the repository has no commits yet",
            quoted(name)
        ),
        InputProblem::Unresolved { name, reference } => format!(
            "input {}: {} does not name a commit",
            quoted(name),
            quoted(reference)
        ),
    }
}

fn fix(workflow: &str, problem: &InputProblem) -> String {
    match problem {
        InputProblem::Unknown { name } => {
            format!("drop --input {name}, or declare it in {WORKFLOWS_DIR}/{workflow}.toml")
        }
        InputProblem::Duplicate { name } => format!("pass --input {name} once"),
        InputProblem::Missing { name } | InputProblem::Empty { name } => {
            format!("pass --input {name}=<value>")
        }
        InputProblem::NotJson { name, .. } => format!("pass JSON as --input {name}=<value>"),
        InputProblem::Unresolved { reference, .. } if reference == HEAD => {
            "git commit --allow-empty -m \"first commit\"".to_owned()
        }
        InputProblem::Unresolved { name, .. } => {
            format!("pass --input {name}=<branch, tag or commit>")
        }
    }
}

fn listed(message: Message, workflow: &str, declared: &[DeclaredInput]) -> Message {
    if declared.is_empty() {
        return message.context(format!("workflow {workflow} takes no inputs"));
    }
    let rows: Vec<[String; 4]> = declared.iter().map(row).collect();
    table::render(&rows).lines().fold(
        message.context(format!("inputs of {workflow}:")),
        |message, line| message.context(format!("{ROW_INDENT}{line}")),
    )
}

fn row(input: &DeclaredInput) -> [String; 4] {
    let need = if input.optional {
        "optional"
    } else {
        "required"
    };
    let default = input
        .default
        .as_ref()
        .map(|default| format!("(default {default})"));
    let description = [input.description.clone(), default]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    [
        input.name.clone(),
        input.format.name().to_owned(),
        need.to_owned(),
        description,
    ]
}
