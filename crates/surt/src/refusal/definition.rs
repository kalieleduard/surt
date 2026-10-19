use surt_engine::project::{InvalidDefinition, Problem, StepField};

use crate::message::Message;
use crate::symbols::ARROW;

const WORKFLOWS_DIR: &str = ".surt/workflows/";
const TEMPLATE_FIX: &str = "surt init --template feature";

pub(super) fn invalid(invalid: &InvalidDefinition) -> Message {
    let path = invalid.file.path();
    let location = match (&invalid.field, &invalid.step, &invalid.problem) {
        (Some(field), _, _) => format!("{path}: {field}"),
        (None, Some(step), _) => format!("{path}: steps.{step}"),
        (
            None,
            None,
            Problem::Syntax {
                line: Some(line), ..
            },
        ) => format!("{path}:{line}"),
        _ => path.clone(),
    };
    let fix = match invalid.problem {
        Problem::StrayAgentFile => "rename or remove",
        Problem::MissingFile => "create",
        _ => "edit",
    };
    Message::new(format!("{location}: {}", describe(&invalid.problem)))
        .way("fix", format!("{fix} {path}"))
}

pub(super) fn unknown_workflow(name: &str, known: &[String]) -> Message {
    let message = Message::new(format!(
        "no workflow is named \"{name}\" in {WORKFLOWS_DIR}"
    ));
    if known.is_empty() {
        return message
            .context(format!("{WORKFLOWS_DIR} holds no workflow"))
            .way("fix", TEMPLATE_FIX);
    }
    message
        .context(format!("workflows: {}", known.join(", ")))
        .way("fix", format!("add {WORKFLOWS_DIR}{name}.toml"))
}

fn describe(problem: &Problem) -> String {
    match problem {
        Problem::Syntax { detail, .. } | Problem::InvalidStep { detail } => detail.clone(),
        Problem::UnsupportedSchema { found } => {
            format!("schema = {found} is not supported; this surt reads schema = 1")
        }
        Problem::NameMismatch { found } => {
            format!("\"{found}\" does not match its file or directory name")
        }
        Problem::MissingKind => "the step has no kind".to_owned(),
        Problem::UnknownKind { kind } => format!("unknown step kind \"{kind}\""),
        Problem::UnknownWorkflow { name } => {
            format!("no workflow is named \"{name}\" in {WORKFLOWS_DIR}")
        }
        Problem::RecursiveWorkflow { name } => {
            format!("workflow \"{name}\" runs itself through a per_item step")
        }
        Problem::UnknownAgent { name } => {
            format!("no agent is named \"{name}\" in .surt/agents/")
        }
        Problem::UnknownOperation { name } => format!("no operation is named \"{name}\""),
        Problem::UnknownStep { field, name } => {
            format!("{} names an unknown step \"{name}\"", field_name(*field))
        }
        Problem::OutOfScope { field, name } => {
            format!("{} names \"{name}\" from another scope", field_name(*field))
        }
        Problem::SharedBodyStep { name } => format!("\"{name}\" is in the body of two loops"),
        Problem::NestedLoop { name } => format!("loop \"{name}\" is inside another loop"),
        Problem::Cycle { path } => format!(
            "the steps form a cycle: {}",
            path.join(&format!(" {ARROW} "))
        ),
        Problem::UntilOutsideBody { name } => {
            format!("until names \"{name}\", which is not in the loop's body")
        }
        Problem::UnknownOutcome {
            step,
            outcome,
            declared,
            ..
        } => format!(
            "expects outcome \"{outcome}\" of \"{step}\", which declares: {}",
            declared.join(", ")
        ),
        Problem::UnknownAgentInput { input } => {
            format!("maps input \"{input}\", which the agent does not declare")
        }
        Problem::MissingInput { input } => {
            format!("needs \"{input}\", which nothing before it produces")
        }
        Problem::UnproducedArtifact { artifact } => {
            format!("\"{artifact}\" is not produced before this step")
        }
        Problem::NotAnItemList { artifact } => format!("\"{artifact}\" is not an item list"),
        Problem::DuplicateRejectionArtifact { artifact } => {
            format!("rejection artifact \"{artifact}\" is declared twice")
        }
        Problem::MissingAsset { path } => {
            format!("\"{path}\" is not a file in the agent's directory")
        }
        Problem::NonLocalReference { reference } => {
            format!("$ref \"{reference}\" is not a file in the agent's schemas directory")
        }
        Problem::InvalidSchema { detail } => format!("not a valid JSON Schema: {detail}"),
        Problem::MisplacedJsonSchema { name } => {
            format!("\"{name}\" has a json_schema but is neither json nor item_list")
        }
        Problem::MisplacedItemFiles { name } => {
            format!("\"{name}\" declares item_files but is not an item_list")
        }
        Problem::UnsupportedItemFileFormat { name } => {
            format!("item file \"{name}\" has an unsupported format")
        }
        Problem::MissingClaudeProfile => {
            "runtime claude-code needs a [profile.claude] table".to_owned()
        }
        Problem::MisplacedApiKeyEnv => "api_key_env needs auth = \"api_key\"".to_owned(),
        Problem::InvalidBudget => "max_budget_usd must be a positive number".to_owned(),
        Problem::MissingFile => "the file does not exist".to_owned(),
        Problem::MissingCheckedOutput { output } => {
            format!("a delivery check needs the JSON output \"{output}\"")
        }
        Problem::MissingCheckedOutcome { outcome } => {
            format!("a delivery check needs the outcome \"{outcome}\"")
        }
        Problem::TestOnlyRuntime => "runtime \"fake\" is test-only".to_owned(),
        Problem::StrayAgentFile => {
            "surt reads only agent.toml, instructions.md and schemas/ here".to_owned()
        }
        Problem::MisplacedInputDefault { input } => {
            format!("input \"{input}\" has a default; only git_ref inputs take one")
        }
        Problem::UnknownSubjectInput { input } => {
            format!("the subject comes from \"{input}\", which is not a declared input")
        }
        Problem::SubjectInputNotText { input, format } => format!(
            "the subject comes from \"{input}\", a {} input; it must be text or markdown",
            format.name()
        ),
        Problem::InputsInChildWorkflow => {
            "a per_item child workflow takes no inputs; it reads item and pass".to_owned()
        }
        Problem::GitInputNotGitRef { input } => {
            format!("needs \"{input}\" from an input with format = \"git_ref\"")
        }
    }
}

fn field_name(field: StepField) -> &'static str {
    match field {
        StepField::Needs => "needs",
        StepField::Body => "body",
        StepField::When => "when",
        StepField::Until => "until",
    }
}
