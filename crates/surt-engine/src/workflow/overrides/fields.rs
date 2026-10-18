use crate::workflow::error::Problem;

const KEY_SEPARATOR: &str = ".";

pub(super) fn field_of(problem: &Problem, merged: &str) -> Option<String> {
    let field = match problem {
        Problem::Syntax {
            line: Some(line), ..
        } => return field_at(merged, *line),
        Problem::UnsupportedSchema { .. } => "schema",
        Problem::NameMismatch { .. } => "id",
        Problem::MissingClaudeProfile => "profile.claude",
        Problem::MisplacedApiKeyEnv => "profile.claude.api_key_env",
        Problem::InvalidBudget => "profile.limits.max_budget_usd",
        _ => return None,
    };
    Some(field.to_owned())
}

pub(super) fn without_line(problem: Problem) -> Problem {
    match problem {
        Problem::Syntax { detail, .. } => Problem::Syntax { line: None, detail },
        other => other,
    }
}

fn field_at(canonical: &str, line: usize) -> Option<String> {
    let mut table: Option<&str> = None;
    for text in canonical.lines().take(line) {
        if let Some(header) = header(text) {
            table = Some(header);
        }
    }
    let target = canonical.lines().nth(line.checked_sub(1)?)?;
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
