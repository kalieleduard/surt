use std::fmt::Write;

use surt_engine::project::{GateKind, GateStatus, GateView, ItemListSummary, RunStatus};

use super::{CONTINUATION, field, line};
use crate::driving::local_time;
use crate::show::origin;
use crate::status::item_count;
use crate::symbols::{DASH, EDITED, ELLIPSIS, ERROR, PAUSED, SEPARATOR, SUCCESS, WARNING};

pub(super) fn render(text: &mut String, status: &RunStatus) {
    for gate in &status.gates {
        let Some(revision) = &gate.revision else {
            continue;
        };
        let state = match gate.kind {
            GateKind::Approval => decision(gate, revision.item_list),
            GateKind::Input => answering(gate),
        };
        let summary = format!(
            "rev {} ({}){state}",
            revision.number,
            origin(revision.origin)
        );
        field(text, &gate.artifact, &summary);
        if gate.kind == GateKind::Approval && waiting(gate) {
            for file in &revision.files {
                let directory = gate.working_copy.display();
                let _ = writeln!(text, "{CONTINUATION}{directory}/{file}");
            }
        }
        changes(text, gate, revision.number);
    }
    let subject = &status.subject;
    if waits(status, GateKind::Approval) {
        line(
            text,
            &format!(
                "approve: surt approve {subject}    reject: surt reject {subject} -m \"{ELLIPSIS}\""
            ),
        );
    }
    if waits(status, GateKind::Input) {
        line(text, &format!("answer: surt answer {subject}"));
    }
}

fn waiting(gate: &GateView) -> bool {
    matches!(gate.status, GateStatus::Waiting { .. })
}

fn waits(status: &RunStatus, kind: GateKind) -> bool {
    status
        .gates
        .iter()
        .any(|gate| gate.kind == kind && waiting(gate))
}

fn answering(gate: &GateView) -> String {
    let round = gate
        .round
        .map(|round| format!(" (round {round})"))
        .unwrap_or_default();
    match gate.status {
        GateStatus::Waiting { .. } => format!(" {PAUSED} awaiting answers{round}"),
        GateStatus::Answered { .. } => {
            format!("{}{round}", decided(gate, &format!(" {SUCCESS} answered")))
        }
        _ => String::new(),
    }
}

fn decided(gate: &GateView, verb: &str) -> String {
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
}

fn decision(gate: &GateView, item_list: Option<ItemListSummary>) -> String {
    let by = |verb: &str| decided(gate, verb);
    let items = item_list
        .map(|list| format!("{SEPARATOR}{}", item_count(list.items)))
        .unwrap_or_default();
    let item_files = item_list.map_or(0, |list| list.item_files);
    match gate.status {
        GateStatus::Approved { .. } => {
            format!("{}{items}", by(&format!(" {SUCCESS} approved")))
        }
        GateStatus::Rejected { .. } => format!("{}{items}", by(&format!(" {ERROR} rejected"))),
        GateStatus::Waiting { .. } if item_files > 0 => format!(
            " {PAUSED} awaiting approval {DASH} approving covers the {} and all {item_files} item files:",
            gate.artifact
        ),
        GateStatus::Waiting { .. } => {
            format!(" {PAUSED} awaiting approval {DASH} approving covers:")
        }
        GateStatus::NotReached | GateStatus::Answered { .. } => String::new(),
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
