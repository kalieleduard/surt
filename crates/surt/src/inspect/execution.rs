mod reported;

use surt_engine::project::{ExecutionDetail, ExecutionInput, Performer};

use super::scope;
use crate::symbols::{DASH, ELLIPSIS, SEPARATOR};
use crate::table;

const SHORT_SHA256: usize = 12;
const INDENT: &str = "  ";

type Field = (&'static str, Option<String>);

pub(crate) fn render(detail: &ExecutionDetail, now: i64) -> String {
    let rows: Vec<[String; 2]> = fields(detail, now)
        .into_iter()
        .filter_map(|(label, value)| {
            let value = value.filter(|value| !value.is_empty())?;
            Some([label.to_owned(), value])
        })
        .collect();
    let body: String = table::render(&rows)
        .lines()
        .map(|line| format!("{INDENT}{line}\n"))
        .collect();
    format!("execution {}\n{body}", detail.id)
}

fn fields(detail: &ExecutionDetail, now: i64) -> Vec<Field> {
    let mut fields = vec![
        ("feature", Some(feature(detail))),
        ("step", Some(step(detail))),
    ];
    fields.extend(performer(detail));
    fields.extend([
        ("session", reported::session(&detail.reported)),
        ("process", reported::process(detail, now)),
        ("usage", reported::usage(&detail.reported)),
        ("denied", reported::denied(&detail.reported)),
    ]);
    fields.extend(detail.inputs.iter().enumerate().map(|(index, input)| {
        let label = if index == 0 { "inputs" } else { "" };
        (label, Some(described(input)))
    }));
    let worktree = detail
        .worktree
        .as_ref()
        .map(|path| path.display().to_string());
    fields.extend([
        ("worktree", worktree),
        ("salvage", detail.salvage_ref.clone()),
        ("logs", Some(format!("surt logs {}", detail.id))),
    ]);
    fields
}

fn performer(detail: &ExecutionDetail) -> [Field; 3] {
    match &detail.performer {
        Performer::Agent { role, profile } => [
            ("role", Some(role.clone())),
            ("state", Some(state(detail))),
            ("profile", Some(reported::profile(profile))),
        ],
        Performer::Operation { id } => [
            ("operation", Some(id.clone())),
            ("state", Some(state(detail))),
            ("profile", None),
        ],
    }
}

fn feature(detail: &ExecutionDetail) -> String {
    let mut parts = vec![detail.subject.clone(), format!("run {}", detail.run)];
    parts.extend(detail.item.as_ref().map(|item| format!("task {item}")));
    parts.join(SEPARATOR)
}

fn step(detail: &ExecutionDetail) -> String {
    let mut parts = scope(
        None,
        Some(&detail.step),
        detail.iteration,
        Some(detail.attempt),
    );
    parts.extend(detail.slot.map(|slot| format!("agent-{slot}")));
    parts.join(SEPARATOR)
}

fn state(detail: &ExecutionDetail) -> String {
    let headline = match (&detail.outcome, &detail.failure) {
        (Some(outcome), _) => format!("{} ({outcome})", detail.state),
        (None, Some(failure)) => format!("{} ({})", detail.state, failure.class),
        (None, None) => detail.state.clone(),
    };
    let error = detail.failure.as_ref().and_then(|failure| {
        let parts: Vec<&str> = [failure.code.as_deref(), failure.message.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        (!parts.is_empty()).then(|| parts.join(&format!(" {DASH} ")))
    });
    [Some(headline), error]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

fn described(input: &ExecutionInput) -> String {
    let revision = format!("{} rev {}", input.artifact, input.revision);
    let name = match &input.item {
        Some(item) => format!("{} {item} ({revision})", input.name),
        None if input.name == input.artifact => revision,
        None => format!("{} ({revision})", input.name),
    };
    let sha = input.sha256.get(..SHORT_SHA256).unwrap_or(&input.sha256);
    format!("{name} sha256:{sha}{ELLIPSIS}")
}

#[cfg(test)]
mod tests;
