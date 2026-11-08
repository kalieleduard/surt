use serde_json::json;
use surt_engine::project::{Questions, WaitingInput};

use super::WaitingInputView;

#[test]
fn waiting_questions_read_with_their_recommendation_and_described_or_plain_options() {
    let questions = Questions::parse(
        json!({
            "questions": [
                {
                    "id": "auth",
                    "question": "Which login?",
                    "why": "It decides the schema.",
                    "recommended": "sso",
                    "options": [
                        { "value": "sso", "description": "Company accounts" },
                        "password"
                    ]
                },
                {
                    "id": "scope",
                    "question": "Anything else?",
                    "why": "Keeps the spec small.",
                    "recommended": "no"
                }
            ]
        })
        .to_string()
        .as_bytes(),
    )
    .expect("the questions parse");
    let waiting = WaitingInput {
        step: "answer-spec".to_owned(),
        artifact: "spec-questions".to_owned(),
        revision: 2,
        round: 1,
        questions,
    };

    let view = serde_json::to_value(WaitingInputView::from(waiting)).expect("serializes");

    assert_eq!(
        view,
        json!({
            "step": "answer-spec",
            "artifact": "spec-questions",
            "revision": 2,
            "round": 1,
            "questions": [
                {
                    "id": "auth", "question": "Which login?", "why": "It decides the schema.",
                    "recommended": "sso",
                    "options": [
                        { "value": "sso", "description": "Company accounts" },
                        { "value": "password", "description": null }
                    ]
                },
                {
                    "id": "scope", "question": "Anything else?", "why": "Keeps the spec small.",
                    "recommended": "no", "options": null
                }
            ]
        })
    );
}
