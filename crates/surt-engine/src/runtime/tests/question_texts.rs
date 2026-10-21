use serde_json::json;

use super::asker::{accepted, problem, question, round, with};
use crate::artifacts::{QuestionField, QuestionsProblem};

fn empty(field: QuestionField) -> QuestionsProblem {
    QuestionsProblem::EmptyText {
        id: "login-id".to_owned(),
        field,
    }
}

fn offering(options: serde_json::Value) -> String {
    round(&[with(question("login-id"), "options", options)])
}

#[tokio::test]
async fn the_question_why_and_recommendation_are_never_blank() {
    let cases = [
        ("question", QuestionField::Question),
        ("why", QuestionField::Why),
        ("recommended", QuestionField::Recommended),
    ];
    for (name, field) in cases {
        for blank in ["", " \n\t"] {
            let document = round(&[with(question("login-id"), name, json!(blank))]);

            assert_eq!(problem(&document).await, empty(field), "{name} {blank:?}");
        }
    }
}

#[tokio::test]
async fn options_are_optional_and_otherwise_two_or_more() {
    let id = || "login-id".to_owned();

    let without = accepted(&round(&[question("login-id")])).await;
    let two = accepted(&offering(json!(["e-mail or username", "e-mail only"]))).await;

    assert_eq!((without.len(), two.len()), (1, 1));
    assert_eq!(
        problem(&offering(json!(["e-mail only"]))).await,
        QuestionsProblem::TooFewOptions { id: id() }
    );
    assert_eq!(
        problem(&offering(json!([]))).await,
        QuestionsProblem::TooFewOptions { id: id() }
    );
}

#[tokio::test]
async fn options_are_non_blank_and_unique() {
    let id = || "login-id".to_owned();

    assert_eq!(
        problem(&offering(json!(["e-mail only", " "]))).await,
        QuestionsProblem::EmptyOption { id: id() }
    );
    assert_eq!(
        problem(&offering(json!(["e-mail only", "username", "e-mail only"]))).await,
        QuestionsProblem::DuplicateOption {
            id: id(),
            option: "e-mail only".to_owned()
        }
    );
}

#[tokio::test]
async fn the_recommendation_is_one_of_the_options() {
    let document = offering(json!(["E-mail only", "e-mail or username"]));

    assert_eq!(
        problem(&document).await,
        QuestionsProblem::RecommendationNotOffered {
            id: "login-id".to_owned()
        }
    );
}

#[tokio::test]
async fn an_option_may_carry_a_description_and_mix_with_plain_options() {
    let document = offering(json!([
        { "value": "e-mail only", "description": "one login field, no username column" },
        "e-mail or username",
    ]));

    assert_eq!(accepted(&document).await.len(), 1);
}

#[tokio::test]
async fn a_described_option_needs_a_non_blank_value_and_description() {
    let id = || "login-id".to_owned();

    assert_eq!(
        problem(&offering(json!([
            { "value": "e-mail only", "description": " " },
            "username",
        ])))
        .await,
        QuestionsProblem::EmptyDescription {
            id: id(),
            option: "e-mail only".to_owned()
        }
    );
    assert_eq!(
        problem(&offering(json!([
            { "value": "", "description": "no login" },
            "e-mail only",
        ])))
        .await,
        QuestionsProblem::EmptyOption { id: id() }
    );
}

#[tokio::test]
async fn a_described_option_naming_an_unknown_field_is_malformed() {
    let document = offering(json!([
        { "value": "e-mail only", "description": "one login field", "label": "E-mail" },
        "username",
    ]));

    assert_eq!(
        problem(&document).await,
        QuestionsProblem::MalformedQuestion { index: 0 }
    );
}

#[tokio::test]
async fn the_recommendation_matches_a_described_option_by_its_value() {
    let document = offering(json!([
        { "value": "e-mail only", "description": "one login field" },
        { "value": "e-mail or username", "description": "two login fields" },
    ]));

    assert_eq!(accepted(&document).await.len(), 1);
}
