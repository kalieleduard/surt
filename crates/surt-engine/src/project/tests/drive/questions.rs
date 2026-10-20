use serde_json::{Value, json};

use super::super::fixtures::{Catalog, SUBJECT};
use super::fixtures::{Feature, Recording, agent};
use super::team::input;
use crate::project::{DriveOutcome, Questions, RunSelector, ShowRequest};
use crate::runtime::{Behavior, Delivery, Script};

const FLOW: &str = r#"
schema = 1
name = "ask"

[inputs.request]
format = "text"

[steps.ask]
kind = "agent"
agent = "asker"

[steps.read]
kind = "agent"
agent = "reader"
needs = ["ask"]
"#;

const ASKER: &str = r#"
schema = 1
id = "asker"

[inputs.request]
format = "text"

[outputs.spec-questions]
format = "questions"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

const READER: &str = r#"
schema = 1
id = "reader"

[inputs.spec-questions]
format = "questions"

[outputs.notes]
format = "text"

[profile]
runtime = "fake"
model = "fake-model"
effort = "high"
"#;

fn round() -> Value {
    json!({ "questions": [
        { "id": "login-id", "question": "Log in by e-mail only, or by username too?",
          "why": "changes the users table", "recommended": "e-mail only",
          "options": ["e-mail only", "e-mail or username"] },
        { "id": "lockout", "question": "Lock an account after failed logins?",
          "why": "adds a column and an error message", "recommended": "after 5 failures" }
    ] })
}

fn script() -> Script {
    let asked =
        Delivery::new("done").output("spec-questions", "questions.json", round().to_string());
    let noted = Delivery::new("done").output("notes", "notes.txt", "noted\n");
    Script::new().on("ask", 1, 1, Behavior::Deliver(asked)).on(
        "read",
        1,
        1,
        Behavior::Deliver(noted),
    )
}

fn shown_questions() -> ShowRequest<'static> {
    ShowRequest {
        artifact: "spec-questions",
        revision: None,
        item: None,
        iteration: None,
        file: None,
        run: RunSelector::Ordinal(1),
    }
}

async fn asked() -> (Feature, Recording) {
    let catalog = Catalog::new()
        .workflow("ask", FLOW)
        .agent("asker", agent(ASKER))
        .agent("reader", agent(READER));
    let feature = Feature::new(catalog, "ask").await;
    let recording = Recording::new(script());
    let driven = feature.drive_with(&recording).await;
    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    (feature, recording)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_step_that_declares_a_questions_input_reads_the_questions_another_step_delivered() {
    let (_feature, recording) = asked().await;

    let requests = recording.requests();
    let reader = requests.last().expect("the reader's Execution");

    assert_eq!(
        input(reader, "spec-questions", "questions.json"),
        Some(round())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_published_round_reads_back_as_typed_questions() {
    let (feature, _) = asked().await;
    let shown = feature
        .project
        .show(SUBJECT, shown_questions())
        .await
        .expect("the questions Revision");

    let questions = Questions::parse(&shown.content).expect("valid questions");

    let read: Vec<_> = questions
        .questions()
        .iter()
        .map(|question| {
            (
                question.id(),
                question.question(),
                question.why(),
                question.recommended(),
                question.options().map(<[String]>::to_vec),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                "login-id",
                "Log in by e-mail only, or by username too?",
                "changes the users table",
                "e-mail only",
                Some(vec![
                    "e-mail only".to_owned(),
                    "e-mail or username".to_owned()
                ])
            ),
            (
                "lockout",
                "Lock an account after failed logins?",
                "adds a column and an error message",
                "after 5 failures",
                None
            ),
        ]
    );
    assert_eq!(Questions::parse(br#"{ "questions": [] }"#), None);
    assert_eq!(Questions::parse(b"not json"), None);
}
