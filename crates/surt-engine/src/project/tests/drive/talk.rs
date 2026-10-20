use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::super::fixtures::Catalog;
use super::fixtures::{Feature, agent, label};
use crate::project::{DriveOutcome, Gate, GateKind};
use crate::runtime::{Behavior, Delivery};

pub(super) const TALK_WORKFLOW: &str = r#"
schema = 1
name = "talk"

[inputs.request]
format = "text"

[steps.spec]
kind = "agent"
agent = "speccer"

[steps.answer-spec]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
needs = ["spec"]
when = { step = "spec", outcome = "questions" }

[steps.approve-spec]
kind = "approval"
artifact = "spec"
rejection_artifact = "spec-rejection"
needs = ["answer-spec"]
when = { step = "spec", outcome = "done" }

[steps.spec-loop]
kind = "loop"
body = ["spec", "answer-spec", "approve-spec"]
until = { step = "approve-spec", outcome = "approved" }
"#;

pub(super) const SPECCER: &str = r#"
schema = 1
id = "speccer"
outcomes = ["done", "questions"]

[inputs.request]
format = "text"

[inputs.spec-answers]
format = "json"
optional = true

[inputs.spec-rejection]
format = "text"
optional = true

[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.spec-questions]
format = "questions"
outcomes = ["questions"]

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

pub(super) fn talk_catalog(workflow: &'static str) -> Catalog {
    Catalog::new()
        .workflow("talk", workflow)
        .agent("speccer", agent(SPECCER))
}

pub(super) async fn talk() -> Feature {
    Feature::new(talk_catalog(TALK_WORKFLOW), "talk").await
}

pub(super) fn questions(ids: &[&str]) -> Value {
    let asked: Vec<Value> = ids
        .iter()
        .map(|id| {
            json!({ "id": id, "question": format!("What about {id}?"),
                    "why": "it changes the spec", "recommended": format!("{id} as usual") })
        })
        .collect();
    json!({ "questions": asked })
}

pub(super) fn ask(ids: &[&str]) -> Behavior {
    let asked = Delivery::new("questions").output(
        "spec-questions",
        "questions.json",
        questions(ids).to_string(),
    );
    Behavior::Deliver(asked)
}

pub(super) fn spec(text: &str) -> Behavior {
    Behavior::Deliver(Delivery::new("done").output("spec", "spec.md", text))
}

pub(super) fn answers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
        .collect()
}

pub(super) fn input_gate(iteration: u32) -> DriveOutcome {
    DriveOutcome::AtGate {
        gates: vec![Gate {
            step: label("answer-spec", Some(iteration)),
            kind: GateKind::Input,
            artifact: "spec-questions".to_owned(),
        }],
    }
}

pub(super) const ASKING_LOOP: &str = r#"
schema = 1
name = "talk"

[inputs.request]
format = "text"

[steps.spec]
kind = "agent"
agent = "speccer"

[steps.answer-spec]
kind = "input"
artifact = "spec-questions"
answer_artifact = "spec-answers"
needs = ["spec"]
when = { step = "spec", outcome = "questions" }

[steps.spec-loop]
kind = "loop"
body = ["spec", "answer-spec"]
until = { step = "spec", outcome = "done" }
"#;

pub(super) async fn undriven(feature: &Feature) {
    feature
        .dir
        .execute("UPDATE workflow_run SET driver_pid = NULL")
        .await;
}

pub(super) async fn answer_revisions(feature: &Feature) -> Vec<(i64,)> {
    feature
        .dir
        .rows(
            "SELECT count(*) FROM artifact_revision r JOIN artifact a ON a.id = r.artifact_id \
             WHERE a.name = 'spec-answers'",
        )
        .await
}
