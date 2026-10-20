use serde_json::Value;
use surt_engine::project::{GateKind, GateStatus, GateView, ItemListSummary};

use crate::json::Fields;
use crate::show::origin;

pub(super) fn gate(view: &GateView) -> Value {
    let (status, decision) = match view.status {
        GateStatus::NotReached => ("not_reached", None),
        GateStatus::Waiting { .. } => ("waiting", None),
        GateStatus::Approved { .. } => ("approved", Some("approve")),
        GateStatus::Rejected { .. } => ("rejected", Some("reject")),
        GateStatus::Answered { .. } => ("answered", None),
    };
    let answered = matches!(view.status, GateStatus::Answered { .. });
    Fields::new()
        .with("step", view.step.as_str())
        .with("kind", gate_kind(view.kind))
        .with("artifact", view.artifact.as_str())
        .with("status", status)
        .maybe("round", view.round)
        .maybe(
            "revision",
            view.revision.as_ref().map(|revision| {
                Fields::new()
                    .with("number", revision.number)
                    .with("origin", origin(revision.origin))
                    .with("sha256", revision.sha256.as_str())
                    .with("files", revision.files.clone())
                    .maybe("item_list", revision.item_list.map(item_list))
            }),
        )
        .maybe(
            "approval",
            decision.map(|decision| {
                Fields::new()
                    .with("decision", decision)
                    .maybe("actor", view.decided_by.as_deref())
                    .time("decided_at", view.decided_at)
            }),
        )
        .maybe(
            "answer",
            answered.then(|| {
                Fields::new()
                    .maybe("actor", view.decided_by.as_deref())
                    .time("answered_at", view.decided_at)
            }),
        )
        .list("edited", view.edited.clone())
        .list("missing", view.missing.clone())
        .into()
}

fn gate_kind(kind: GateKind) -> &'static str {
    match kind {
        GateKind::Approval => "approval",
        GateKind::Input => "input",
    }
}

fn item_list(list: ItemListSummary) -> Fields {
    Fields::new()
        .with("items", list.items)
        .with("item_files", list.item_files)
}
