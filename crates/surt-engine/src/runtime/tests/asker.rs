use serde_json::{Value, json};

use super::fixtures::{Execution, contract};
use crate::artifacts::{DeliveryContract, DeliveryRejection, QuestionsProblem};
use crate::runtime::{Behavior, Delivery, Verdict};

pub(super) const PATH: &str = "questions.json";

const ASKER: &str = r#"
schema = 1
id = "asker"

[outputs.spec-questions]
format = "questions"

[profile]
runtime = "fake"
model = "claude-sonnet-5-5"
effort = "high"
"#;

fn asker() -> DeliveryContract {
    contract("asker", ASKER, &[])
}

pub(super) fn question(id: &str) -> Value {
    json!({
        "id": id,
        "question": "Log in by e-mail only, or by username too?",
        "why": "changes the users table",
        "recommended": "e-mail only",
    })
}

pub(super) fn with(mut question: Value, field: &str, value: Value) -> Value {
    question[field] = value;
    question
}

pub(super) fn round(questions: &[Value]) -> String {
    json!({ "questions": questions }).to_string()
}

pub(super) fn asking(document: &str) -> Delivery {
    Delivery::new("done").output("spec-questions", PATH, document)
}

pub(super) async fn accepted(document: &str) -> Vec<(String, Vec<u8>)> {
    let verdict = Execution::with_contract(asker())
        .run(Behavior::Deliver(asking(document)))
        .await
        .expect("the fake completes");
    let Verdict::Accepted(mut delivery) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    let output = delivery
        .outputs
        .remove("spec-questions")
        .expect("the questions output");
    output
        .files
        .into_iter()
        .map(|file| (file.path, file.bytes))
        .collect()
}

pub(super) async fn rejection(document: &str) -> DeliveryRejection {
    Execution::with_contract(asker())
        .delivery_rejection(asking(document))
        .await
}

pub(super) async fn problem(document: &str) -> QuestionsProblem {
    let rejection = rejection(document).await;
    let DeliveryRejection::InvalidQuestions { path, problem } = rejection else {
        panic!("expected invalid questions, got {rejection:?}");
    };
    assert_eq!(path, PATH);
    problem
}
