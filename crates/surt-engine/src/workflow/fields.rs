use crate::workflow::error::Problem;

const KEY_SEPARATOR: &str = ".";
const OUTPUTS: &str = "outputs";
const FORMAT: &str = "format";
const OUTCOMES: &str = "outcomes";
pub(super) const INPUTS: &str = "inputs";
const DEFAULT: &str = "default";
const SUBJECT_FROM: &str = "subject.from";

pub(super) fn agent_field(problem: &Problem, text: &str) -> Option<String> {
    let field = match problem {
        Problem::NameMismatch { .. } => "id",
        Problem::MissingClaudeProfile => "profile.claude",
        Problem::MisplacedApiKeyEnv => "profile.claude.api_key_env",
        Problem::InvalidBudget => "profile.limits.max_budget_usd",
        Problem::TestOnlyRuntime => "profile.runtime",
        Problem::MissingCheckedOutcome { .. } => OUTCOMES,
        Problem::MissingCheckedOutput { output } => {
            return Some([OUTPUTS, output, FORMAT].join(KEY_SEPARATOR));
        }
        Problem::UndeliveredCheckedOutput { output, .. }
        | Problem::EmptyOutputOutcomes { output }
        | Problem::UnknownOutputOutcome { output, .. }
        | Problem::DuplicateOutputOutcome { output, .. } => {
            return Some([OUTPUTS, output, OUTCOMES].join(KEY_SEPARATOR));
        }
        _ => return common_field(problem, text),
    };
    Some(field.to_owned())
}

pub(super) fn workflow_field(problem: &Problem, text: &str) -> Option<String> {
    match problem {
        Problem::NameMismatch { .. } => Some("name".to_owned()),
        Problem::MisplacedInputDefault { input } => {
            Some([INPUTS, input, DEFAULT].join(KEY_SEPARATOR))
        }
        Problem::UnknownSubjectInput { .. } | Problem::SubjectInputNotText { .. } => {
            Some(SUBJECT_FROM.to_owned())
        }
        _ => common_field(problem, text),
    }
}

fn common_field(problem: &Problem, text: &str) -> Option<String> {
    match problem {
        Problem::Syntax {
            line: Some(line), ..
        } => field_at(text, *line),
        Problem::UnsupportedSchema { .. } => Some("schema".to_owned()),
        _ => None,
    }
}

fn field_at(text: &str, line: usize) -> Option<String> {
    let mut table: Option<&str> = None;
    for text in text.lines().take(line) {
        if let Some(header) = header(text) {
            table = Some(header);
        }
    }
    let target = text.lines().nth(line.checked_sub(1)?)?;
    if header(target).is_some() {
        return table.map(str::to_owned);
    }
    let key = target.split_once('=')?.0.trim();
    Some(match table {
        Some(table) => [table, key].join(KEY_SEPARATOR),
        None => key.to_owned(),
    })
}

fn header(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
}
