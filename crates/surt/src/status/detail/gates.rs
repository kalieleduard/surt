use std::fmt::Write;

use surt_engine::project::{GateStatus, GateView, RunStatus};

use super::{CONTINUATION, field, line};
use crate::driving::local_time;
use crate::show::origin;
use crate::status::item_files;
use crate::symbols::{DASH, EDITED, ELLIPSIS, ERROR, PAUSED, SEPARATOR, SUCCESS, WARNING};

pub(super) fn render(text: &mut String, status: &RunStatus) {
    for gate in &status.gates {
        let Some(revision) = &gate.revision else {
            continue;
        };
        let summary = format!(
            "rev {} ({}){}",
            revision.number,
            origin(revision.origin),
            decision(gate, item_files(revision.files.len()))
        );
        field(text, &gate.artifact, &summary);
        if matches!(gate.status, GateStatus::Waiting { .. }) {
            for file in &revision.files {
                let directory = gate.working_copy.display();
                let _ = writeln!(text, "{CONTINUATION}{directory}/{file}");
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
            &format!(
                "approve: surt approve {subject}    reject: surt reject {subject} -m \"{ELLIPSIS}\""
            ),
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
        (GateStatus::Approved { .. }, _) => {
            format!("{}{items}", by(&format!(" {SUCCESS} approved")))
        }
        (GateStatus::Rejected { .. }, _) => format!("{}{items}", by(&format!(" {ERROR} rejected"))),
        (GateStatus::Waiting { .. }, Some(count)) => format!(
            " {PAUSED} awaiting approval {DASH} approving covers the {} and all {count} task files:",
            gate.artifact
        ),
        (GateStatus::Waiting { .. }, None) => {
            format!(" {PAUSED} awaiting approval {DASH} approving covers:")
        }
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
        let _ = writeln!(text, "{CONTINUATION}{EDITED} {file} edited {DASH} {effect}");
    }
    for file in &gate.missing {
        let _ = writeln!(
            text,
            "{CONTINUATION}{WARNING} {file} missing from the Working Copy"
        );
    }
}

fn tasks(count: usize) -> String {
    match count {
        1 => format!("{SEPARATOR}1 task"),
        _ => format!("{SEPARATOR}{count} tasks"),
    }
}
