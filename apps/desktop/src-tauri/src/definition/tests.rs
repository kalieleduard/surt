use serde_json::{Map, Value, json};
use surt_engine::project::{
    DefinedStep, DefinedStepKind, FrozenDefinition, StepCondition, Timeline, TimelineEvent,
};

use super::RunDetail;

fn step(key: &str, needs: &[&str], kind: DefinedStepKind) -> DefinedStep {
    DefinedStep {
        key: key.to_owned(),
        needs: needs.iter().map(|need| (*need).to_owned()).collect(),
        when: None,
        kind,
    }
}

fn condition(step: &str, outcome: &str) -> StepCondition {
    StepCondition {
        step: step.to_owned(),
        outcome: outcome.to_owned(),
    }
}

fn data(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

fn feature() -> FrozenDefinition {
    FrozenDefinition {
        name: "feature".to_owned(),
        source: "schema = 1\n".to_owned(),
        steps: vec![
            step(
                "spec",
                &[],
                DefinedStepKind::Agent {
                    agent: "spec".to_owned(),
                },
            ),
            DefinedStep {
                when: Some(condition("spec", "questions")),
                ..step(
                    "answer-spec",
                    &["spec"],
                    DefinedStepKind::Input {
                        artifact: "spec-questions".to_owned(),
                    },
                )
            },
            step(
                "spec-loop",
                &[],
                DefinedStepKind::Loop {
                    body: vec!["spec".to_owned(), "answer-spec".to_owned()],
                    until: condition("spec", "ready"),
                },
            ),
            step(
                "approve-spec",
                &["spec-loop"],
                DefinedStepKind::Approval {
                    artifact: "spec".to_owned(),
                },
            ),
            step(
                "implement",
                &["approve-spec"],
                DefinedStepKind::PerItem {
                    items: "tasks".to_owned(),
                    workflow: "task-cycle".to_owned(),
                },
            ),
            step(
                "merge",
                &["implement"],
                DefinedStepKind::Operation {
                    run: "merge".to_owned(),
                },
            ),
        ],
    }
}

#[test]
fn a_run_detail_names_each_step_kind_the_way_the_canvas_reads_it() {
    let timeline = Timeline {
        subject: "20261107-login".to_owned(),
        run: 1,
        item: None,
        events: vec![TimelineEvent {
            at: 1_800_000_000,
            kind: "step.done".to_owned(),
            item: None,
            step: Some("spec".to_owned()),
            iteration: Some(2),
            attempt: None,
            execution: None,
            data: data(json!({ "outcome": "ready" })),
        }],
    };

    let detail = serde_json::to_value(RunDetail::of(feature(), timeline)).expect("serializes");

    assert_eq!(
        detail,
        json!({
            "definition": {
                "name": "feature",
                "source": "schema = 1\n",
                "steps": [
                    { "key": "spec", "needs": [], "when": null, "kind": "agent", "agent": "spec" },
                    {
                        "key": "answer-spec", "needs": ["spec"],
                        "when": { "step": "spec", "outcome": "questions" },
                        "kind": "input", "artifact": "spec-questions"
                    },
                    {
                        "key": "spec-loop", "needs": [], "when": null, "kind": "loop",
                        "body": ["spec", "answer-spec"], "until": { "step": "spec", "outcome": "ready" }
                    },
                    { "key": "approve-spec", "needs": ["spec-loop"], "when": null, "kind": "approval", "artifact": "spec" },
                    {
                        "key": "implement", "needs": ["approve-spec"], "when": null,
                        "kind": "per_item", "items": "tasks", "workflow": "task-cycle"
                    },
                    { "key": "merge", "needs": ["implement"], "when": null, "kind": "operation", "operation": "merge" }
                ]
            },
            "events": [{
                "at": 1_800_000_000, "kind": "step.done", "item": null, "step": "spec",
                "iteration": 2, "attempt": null, "execution": null, "data": { "outcome": "ready" }
            }]
        })
    );
}
