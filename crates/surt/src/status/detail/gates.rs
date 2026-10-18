use std::fmt::Write;

use surt_engine::project::{GateStatus, GateView, RunStatus};

use super::{CONTINUATION, field, line};
use crate::driving::local_time;
use crate::show::origin;

pub(super) fn render(text: &mut String, status: &RunStatus) {
    for gate in &status.gates {
        let Some(revision) = &gate.revision else {
            continue;
        };
        let summary = format!(
            "rev {} ({}){}",
            revision.number,
            origin(revision.origin),
            decision(gate)
        );
        field(text, &gate.artifact, &summary);
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

fn decision(gate: &GateView) -> String {
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
    match gate.status {
        GateStatus::Approved { .. } => by(" ✔ approved"),
        GateStatus::Rejected { .. } => by(" ✖ rejected"),
        GateStatus::Waiting { .. } => " ⏸ awaiting approval — approving covers:".to_owned(),
        GateStatus::NotReached => String::new(),
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
