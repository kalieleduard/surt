use serde_json::{Value, json};

use crate::answering::{asking, subject_name, surt};

fn stdout(args: &[&str]) -> String {
    let sandbox = asking(&[]);
    let run = surt(&sandbox, args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    run.stdout
}

#[test]
fn status_shows_the_run_awaiting_an_answer_with_the_questions_round_and_how_to_answer() {
    let name = subject_name();

    let detail = stdout(&["status"]);

    assert!(
        detail.starts_with(&format!("{name} · run 1   AWAITING ANSWER")),
        "{detail}"
    );
    assert!(
        detail.contains(&format!(
            "  discovery-questions rev 1 (agent) ⏸ awaiting answers (round 1)\n  \
             answer: surt answer {name}\n"
        )),
        "{detail}"
    );
}

#[test]
fn status_json_lists_the_waiting_input_step_with_its_kind_and_round() {
    let document: Value =
        serde_json::from_str(&stdout(&["status", "--json"])).expect("status is JSON");

    let gates: Vec<Value> = document["gates"]
        .as_array()
        .expect("gates")
        .iter()
        .map(|gate| {
            json!({
                "step": gate["step"],
                "kind": gate["kind"],
                "artifact": gate["artifact"],
                "status": gate["status"],
                "round": gate["round"],
            })
        })
        .collect();

    assert_eq!(document["state"], "AWAITING ANSWER");
    assert_eq!(
        gates,
        [
            json!({ "step": "answer-discovery", "kind": "input", "artifact": "discovery-questions", "status": "waiting", "round": 1 }),
        ]
    );
}
