use serde_json::{Value, json};
use surt_engine::project::{GateKind, GateStatus, GateView, RevisionOrigin, RunState, RunStatus};

use super::super::{detail, json};
use super::{SUBJECT, gate, stopped_with_everything_pending};

fn input(status: GateStatus, round: u32) -> GateView {
    let mut view = GateView {
        kind: GateKind::Input,
        round: Some(round),
        ..gate("answer-spec", "spec-questions", status, &[])
    };
    if let Some(revision) = view.revision.as_mut() {
        revision.origin = RevisionOrigin::Agent;
    }
    view
}

fn awaiting_answer(gates: Vec<GateView>) -> RunStatus {
    RunStatus {
        states: vec![RunState::AwaitingAnswer],
        gates,
        base: None,
        item_step: None,
        items: Vec::new(),
        blocked: Vec::new(),
        unattended: Vec::new(),
        integrating: Vec::new(),
        ..stopped_with_everything_pending()
    }
}

#[test]
fn a_waiting_input_step_shows_its_questions_round_and_how_to_answer() {
    let answered = GateView {
        decided_by: Some("kaliel".to_owned()),
        ..input(GateStatus::Answered { revision: 1 }, 1)
    };
    let status = awaiting_answer(vec![
        answered,
        input(GateStatus::Waiting { revision: 1 }, 2),
    ]);

    let text = detail::render(&status);

    assert_eq!(
        text,
        format!(
            "{SUBJECT} · run 2 of 3   AWAITING ANSWER   (no driver)\n  \
             spec-questions rev 1 (agent) ✔ answered by kaliel (round 1)\n  \
             spec-questions rev 1 (agent) ⏸ awaiting answers (round 2)\n  \
             answer: surt answer {SUBJECT}\n"
        )
    );
}

#[test]
fn status_json_lists_an_input_step_with_its_kind_status_and_round() {
    let answered = GateView {
        decided_by: Some("kaliel".to_owned()),
        ..input(GateStatus::Answered { revision: 1 }, 1)
    };
    let status = awaiting_answer(vec![
        answered,
        input(GateStatus::Waiting { revision: 1 }, 2),
        input(GateStatus::NotReached, 1),
    ]);

    let fields = Value::from(json::subject(&status));
    let gates: Vec<Value> = fields["gates"]
        .as_array()
        .expect("the gates")
        .iter()
        .map(|gate| {
            json!({
                "kind": gate["kind"],
                "status": gate["status"],
                "round": gate["round"],
                "answer": gate["answer"],
            })
        })
        .collect();

    assert_eq!(fields["state"], "AWAITING ANSWER");
    assert_eq!(
        gates,
        [
            json!({ "kind": "input", "status": "answered", "round": 1, "answer": { "actor": "kaliel" } }),
            json!({ "kind": "input", "status": "waiting", "round": 2, "answer": null }),
            json!({ "kind": "input", "status": "not_reached", "round": 1, "answer": null }),
        ]
    );
}

#[test]
fn an_approval_gate_entry_names_its_kind() {
    let fields = Value::from(json::subject(&stopped_with_everything_pending()));

    assert_eq!(fields["gates"][0]["kind"], "approval");
    assert_eq!(fields["gates"][0]["round"], Value::Null);
}
