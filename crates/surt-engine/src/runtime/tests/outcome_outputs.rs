use super::fixtures::{Execution, contract};
use crate::artifacts::{DeliveryContract, DeliveryRejection};
use crate::runtime::{Behavior, Delivery, Verdict};

const SPEC: &str = "# Spec\n";
const QUESTIONS: &str = r#"{ "questions": [] }"#;
const NOTES: &str = "notes\n";

const SPECCER: &str = r#"
schema = 1
id = "speccer"
outcomes = ["done", "questions"]

[outputs.spec]
format = "markdown"
outcomes = ["done"]

[outputs.spec-questions]
format = "json"
outcomes = ["questions"]

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "claude-sonnet-5-5"
effort = "high"
"#;

fn speccer() -> DeliveryContract {
    contract("speccer", SPECCER, &[])
}

fn done() -> Delivery {
    Delivery::new("done")
        .output("spec", "spec.md", SPEC)
        .output("notes", "notes.txt", NOTES)
}

fn questions() -> Delivery {
    Delivery::new("questions")
        .output("spec-questions", "questions.json", QUESTIONS)
        .output("notes", "notes.txt", NOTES)
}

async fn delivered(delivery: Delivery) -> (String, Vec<String>) {
    let verdict = Execution::with_contract(speccer())
        .run(Behavior::Deliver(delivery))
        .await
        .expect("the fake completes");
    let Verdict::Accepted(accepted) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    (accepted.outcome, accepted.outputs.into_keys().collect())
}

async fn rejection(delivery: Delivery) -> DeliveryRejection {
    Execution::with_contract(speccer())
        .delivery_rejection(delivery)
        .await
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[tokio::test]
async fn each_outcome_delivers_exactly_its_own_outputs() {
    let on_done = delivered(done()).await;
    let on_questions = delivered(questions()).await;

    assert_eq!(on_done, ("done".to_owned(), names(&["notes", "spec"])));
    assert_eq!(
        on_questions,
        ("questions".to_owned(), names(&["notes", "spec-questions"]))
    );
}

#[tokio::test]
async fn an_output_the_delivered_outcome_names_is_required() {
    let without_spec = Delivery::new("done").output("notes", "notes.txt", NOTES);
    let without_notes =
        Delivery::new("questions").output("spec-questions", "questions.json", QUESTIONS);

    assert_eq!(
        rejection(without_spec).await,
        DeliveryRejection::MissingOutput {
            name: "spec".to_owned()
        }
    );
    assert_eq!(
        rejection(without_notes).await,
        DeliveryRejection::MissingOutput {
            name: "notes".to_owned()
        }
    );
}

#[tokio::test]
async fn an_output_of_another_outcome_is_not_declared_for_the_delivered_one() {
    let spec_on_questions = questions().output("spec", "spec.md", SPEC);
    let questions_on_done = done().output("spec-questions", "questions.json", QUESTIONS);

    assert_eq!(
        rejection(spec_on_questions).await,
        DeliveryRejection::UndeclaredOutput {
            name: "spec".to_owned()
        }
    );
    assert_eq!(
        rejection(questions_on_done).await,
        DeliveryRejection::UndeclaredOutput {
            name: "spec-questions".to_owned()
        }
    );
}
