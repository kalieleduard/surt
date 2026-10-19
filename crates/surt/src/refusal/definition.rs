use surt_engine::project::{Problem, StepField};

use crate::symbols::ARROW;

pub(super) fn describe(problem: &Problem) -> String {
    match problem {
        Problem::Syntax {
            line: Some(line),
            detail,
        } => format!("line {line}: {detail}"),
        Problem::Syntax { line: None, detail } | Problem::InvalidStep { detail } => detail.clone(),
        Problem::UnsupportedSchema { found } => {
            format!("schema = {found} is not supported; this surt reads schema = 1")
        }
        Problem::NameMismatch { found } => format!("it is named \"{found}\" inside"),
        Problem::MissingKind => "the step has no kind".to_owned(),
        Problem::UnknownKind { kind } => format!("unknown step kind \"{kind}\""),
        Problem::UnknownWorkflow { name } => format!("no workflow is named \"{name}\""),
        Problem::RecursiveWorkflow { name } => {
            format!("workflow \"{name}\" runs itself through a per_item step")
        }
        Problem::UnknownAgent { name } => format!("no agent is named \"{name}\""),
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
            format!("\"{path}\" is neither a local file nor bundled with the agent")
        }
        Problem::NonLocalReference { reference } => {
            format!("$ref \"{reference}\" is not a file in the agent's schemas directory")
        }
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
        Problem::UnknownAgentDirectory { known } => {
            format!("not a built-in agent; agents: {}", known.join(", "))
        }
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
