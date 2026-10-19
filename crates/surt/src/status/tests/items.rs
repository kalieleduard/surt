use serde_json::{Value, json};
use surt_engine::project::{GateStatus, ItemState, RunState, RunStatus};

use super::super::{detail, json, overview};
use super::{SUBJECT, gate_over, item, listed, stopped_with_everything_pending};

fn waiting_on(files: &[&str], item_list: Option<(usize, usize)>) -> RunStatus {
    let gate = gate_over(
        "approve-plan",
        "plan",
        GateStatus::Waiting { revision: 1 },
        &[],
        files,
    );
    let gate = match item_list {
        Some((items, item_files)) => listed(gate, items, item_files),
        None => gate,
    };
    RunStatus {
        states: vec![RunState::AwaitingApproval],
        gates: vec![gate],
        items: Vec::new(),
        blocked: Vec::new(),
        unattended: Vec::new(),
        integrating: Vec::new(),
        ..stopped_with_everything_pending()
    }
}

#[test]
fn the_items_section_is_titled_by_the_per_item_step_of_the_definition() {
    let status = RunStatus {
        item_step: Some("each-chapter".to_owned()),
        items: vec![item("intro", ItemState::Succeeded)],
        ..stopped_with_everything_pending()
    };

    let text = detail::render(&status);

    assert!(
        text.contains("\n  each-chapter\n    intro  ✔ merged    iter 2\n"),
        "{text}"
    );
}

#[test]
fn a_waiting_item_list_counts_items_and_item_files_from_the_engine_not_from_its_files() {
    let files = ["plan.json", "chapters/intro.md", "chapters/intro.png"];

    let status = waiting_on(&files, Some((1, 2)));

    let overview = overview::render(std::slice::from_ref(&status));
    let text = detail::render(&status);

    assert!(overview.contains("plan rev 1 (1 item)\n"), "{overview}");
    assert!(
        text.contains("plan     rev 1 (human) ⏸ awaiting approval — approving covers the plan and all 2 item files:\n"),
        "{text}"
    );
}

#[test]
fn a_waiting_artifact_with_several_files_that_is_no_item_list_counts_no_items() {
    let status = waiting_on(&["notes.md", "figure.png"], None);

    let overview = overview::render(std::slice::from_ref(&status));
    let text = detail::render(&status);

    assert!(overview.contains("plan rev 1\n"), "{overview}");
    assert!(!overview.contains("item"), "{overview}");
    assert!(
        text.contains("plan     rev 1 (human) ⏸ awaiting approval — approving covers:\n"),
        "{text}"
    );
}

#[test]
fn status_json_names_subjects_items_the_item_step_and_the_item_list() {
    let status = waiting_on(&["plan.json", "tasks/A.md"], Some((1, 1)));
    let all = Value::from(json::subjects(std::slice::from_ref(&status)));
    let one = Value::from(json::subject(&status));
    let pending = Value::from(json::subject(&stopped_with_everything_pending()));

    assert_eq!(all["subjects"][0]["name"], json!(SUBJECT));
    assert_eq!(all.get("features"), None);
    assert_eq!(one["item_step"], json!("implement"));
    assert_eq!(
        one["gates"][0]["revision"]["item_list"],
        json!({"items": 1, "item_files": 1})
    );
    assert_eq!(pending["items"][0]["key"], json!("A"));
    assert_eq!(pending["blocked"][0]["item"], json!("B"));
    assert_eq!(pending.get("tasks"), None);
}
