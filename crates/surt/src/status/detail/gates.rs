use std::fmt::Write;

use surt_engine::project::{GateStatus, GateView, RunStatus};

use super::{CONTINUATION, field, line};
use crate::driving::local_time;
use crate::phase::Phase;
use crate::show::origin;

const INDEX_FILES: usize = 1;

pub(super) fn render(text: &mut String, status: &RunStatus) {
    for gate in &status.gates {
        let Some(revision) = &gate.revision else {
            continue;
        };
        let phase = Phase::of_gate(&gate.step);
        let label = phase.map_or(gate.artifact.as_str(), |phase| phase.name());
        let artifact = if label == gate.artifact {
            String::new()
        } else {
            format!("{} ", gate.artifact)
        };
        let covered = phase
            .filter(|phase| phase.covers_item_files())
            .map(|_| revision.files.len().saturating_sub(INDEX_FILES));
        let summary = format!(
            "{artifact}rev {} ({}){}",
            revision.number,
            origin(revision.origin),
            decision(gate, covered)
        );
        field(text, label, &summary);
        if matches!(gate.status, GateStatus::Waiting { .. }) {
            for file in &revision.files {
                let path = gate.working_copy.join(file);
                let _ = writeln!(text, "{CONTINUATION}{}", path.display());
            }
        }
        changes(text, gate, revision.number);
    }
    if status
        .gates
        .iter()
        .any(|gate| matches!(gate.status, GateStatus::Waiting { .. }))
    {
        let subject = &status.subject;
        line(
            text,
            &format!("approve: surt approve {subject}    reject: surt reject {subject} -m \"…\""),
        );
    }
}

fn decision(gate: &GateView, covered: Option<usize>) -> String {
    let by = |verb: &str| {
        let actor = gate
            .decided_by
            .as_deref()
            .map(|actor| format!(" by {actor}"))
            .unwrap_or_default();
        let at = gate
            .decided_at
            .and_then(local_time)
            .map(|time| format!(" {time}"))
            .unwrap_or_default();
        format!("{verb}{actor}{at}")
    };
    let items = covered.map(tasks).unwrap_or_default();
    match (gate.status, covered) {
        (GateStatus::Approved { .. }, _) => format!("{}{items}", by(" ✔ approved")),
        (GateStatus::Rejected { .. }, _) => format!("{}{items}", by(" ✖ rejected")),
        (GateStatus::Waiting { .. }, Some(count)) => format!(
            " ⏸ awaiting approval — approving covers the {} and all {count} task files:",
            gate.artifact
        ),
        (GateStatus::Waiting { .. }, None) => " ⏸ awaiting approval — approving covers:".to_owned(),
        (GateStatus::NotReached, _) => String::new(),
    }
}

fn changes(text: &mut String, gate: &GateView, revision: u32) {
    let next = revision + 1;
    for file in &gate.edited {
        let effect = if matches!(gate.status, GateStatus::Waiting { .. }) {
            format!("recorded as rev {next} on approve (approval then targets rev {next})")
        } else {
            "not recorded".to_owned()
        };
        let _ = writeln!(text, "{CONTINUATION}✎ {file} edited — {effect}");
    }
    for file in &gate.missing {
        let _ = writeln!(text, "{CONTINUATION}⚠ {file} missing from the Working Copy");
    }
}

fn tasks(count: usize) -> String {
    match count {
        1 => " · 1 task".to_owned(),
        _ => format!(" · {count} tasks"),
    }
}
