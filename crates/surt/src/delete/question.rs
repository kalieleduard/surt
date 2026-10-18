use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use surt_engine::project::{BranchFate, DeleteImpact, Directory};

use crate::status::state_name;
use crate::symbols::SEPARATOR;

const LABEL_WIDTH: usize = 9;
const VERB_WIDTH: usize = 6;
const NOTHING: &str = "nothing";
const EXECUTIONS: &str = ".surt/runtime/executions";
const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
const UNIT_STEP: u64 = 1024;

pub(super) fn question(impact: &DeleteImpact) -> String {
    let mut text = format!("delete feature {} and all its history?\n", impact.subject);
    field(&mut text, "runs", &[runs(impact)]);
    field(&mut text, "git", &git(impact));
    field(&mut text, "disk", &[disk(impact)]);
    field(&mut text, "database", &[database(impact)]);
    text
}

pub(super) fn size(bytes: u64) -> String {
    let mut value = bytes;
    let mut unit = 0;
    while value >= UNIT_STEP && unit + 1 < UNITS.len() {
        value /= UNIT_STEP;
        unit += 1;
    }
    format!("{value} {}", UNITS[unit])
}

fn field(text: &mut String, label: &str, lines: &[String]) {
    for (index, line) in lines.iter().enumerate() {
        let label = if index == 0 { label } else { "" };
        let _ = writeln!(text, "  {label:<LABEL_WIDTH$} {line}");
    }
}

fn runs(impact: &DeleteImpact) -> String {
    let runs: Vec<String> = impact
        .runs
        .iter()
        .map(|run| format!("{} {}", run.run, state_name(run.state).to_lowercase()))
        .collect();
    runs.join(SEPARATOR)
}

fn git(impact: &DeleteImpact) -> Vec<String> {
    let (kept, deleted): (Vec<&BranchFate>, Vec<&BranchFate>) = impact
        .integration_branches
        .iter()
        .partition(|fate| fate.kept);
    let mut lines: Vec<String> = deleted
        .iter()
        .map(|fate| {
            verb(
                "delete",
                &format!("{} (run {}, cancelled)", fate.branch, fate.run),
            )
        })
        .collect();
    let mut refs = grouped(&impact.task_branches);
    refs.extend(counted(impact.salvage_refs, "salvage ref", "salvage refs"));
    if !refs.is_empty() {
        lines.push(verb("delete", &refs.join(SEPARATOR)));
    }
    lines.extend(kept.iter().map(|fate| {
        verb(
            "keep",
            &format!("{} (run {}, delivered)", fate.branch, fate.run),
        )
    }));
    if lines.is_empty() {
        lines.push(NOTHING.to_owned());
    }
    lines
}

fn verb(verb: &str, what: &str) -> String {
    format!("{verb:<VERB_WIDTH$} {what}")
}

fn grouped(branches: &[String]) -> Vec<String> {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for branch in branches {
        let (prefix, key) = branch.rsplit_once('/').unwrap_or(("", branch));
        groups.entry(prefix).or_default().push(key);
    }
    groups
        .into_iter()
        .map(|(prefix, keys)| match keys.as_slice() {
            [key] => format!("{prefix}/{key}"),
            _ => format!("{prefix}/{{{}}}", keys.join(",")),
        })
        .collect()
}

fn disk(impact: &DeleteImpact) -> String {
    let (executions, others): (Vec<&Directory>, Vec<&Directory>) = impact
        .directories
        .iter()
        .partition(|directory| directory.path.starts_with(Path::new(EXECUTIONS)));
    let mut parts: Vec<String> = others
        .iter()
        .map(|directory| format!("{} ({})", directory.path.display(), size(directory.bytes)))
        .collect();
    if let Some(dirs) = counted(
        executions.len(),
        "execution directory",
        "execution directories",
    ) {
        let bytes = executions.iter().map(|directory| directory.bytes).sum();
        parts.push(format!("{dirs} ({})", size(bytes)));
    }
    if let Some(edits) = counted(
        impact.unrecorded_edits.len(),
        "unrecorded edit",
        "unrecorded edits",
    ) {
        parts.push(format!("{edits} in the working copy"));
    }
    if parts.is_empty() {
        return NOTHING.to_owned();
    }
    parts.join(SEPARATOR)
}

fn database(impact: &DeleteImpact) -> String {
    [
        (impact.revisions, "revision", "revisions"),
        (impact.executions, "execution", "executions"),
        (impact.events, "event", "events"),
    ]
    .into_iter()
    .filter_map(|(count, one, many)| counted(count, one, many))
    .collect::<Vec<_>>()
    .join(SEPARATOR)
}

fn counted(count: usize, one: &str, many: &str) -> Option<String> {
    match count {
        0 => None,
        1 => Some(format!("1 {one}")),
        _ => Some(format!("{count} {many}")),
    }
}
