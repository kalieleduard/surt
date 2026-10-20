use serde_json::json;

use super::asker::{PATH, accepted, problem, question, rejection, round, with};
use crate::artifacts::{MAX_QUESTIONS, QuestionsProblem};

#[tokio::test]
async fn a_round_of_questions_is_published_as_its_one_file() {
    let document = round(&[with(
        question("login-id"),
        "options",
        json!(["e-mail only", "e-mail or username"]),
    )]);

    let files = accepted(&document).await;

    assert_eq!(files, [(PATH.to_owned(), document.into_bytes())]);
}

#[tokio::test]
async fn a_round_holds_one_to_five_questions() {
    let ids = ["q1", "q2", "q3", "q4", "q5", "q6"];
    let five: Vec<_> = ids[..MAX_QUESTIONS].iter().map(|id| question(id)).collect();
    let six: Vec<_> = ids.iter().map(|id| question(id)).collect();

    let accepted_five = accepted(&round(&five)).await;

    assert_eq!(accepted_five.len(), 1);
    assert_eq!(problem(&round(&[])).await, QuestionsProblem::NoQuestions);
    assert_eq!(
        problem(&round(&six)).await,
        QuestionsProblem::TooManyQuestions { count: 6 }
    );
}

#[tokio::test]
async fn a_file_that_is_not_an_object_holding_a_list_of_questions_is_rejected() {
    for document in [
        "[]",
        r#"{ "asks": [] }"#,
        r#"{ "questions": "login-id" }"#,
        json!({ "questions": [question("login-id")], "note": "x" })
            .to_string()
            .as_str(),
    ] {
        assert_eq!(
            problem(document).await,
            QuestionsProblem::NotQuestions,
            "{document}"
        );
    }
}

#[tokio::test]
async fn a_question_missing_a_field_or_naming_an_unknown_one_is_malformed() {
    let mut missing = question("b");
    missing
        .as_object_mut()
        .expect("a question object")
        .remove("recommended");
    let unknown = with(question("b"), "recommendation", json!("e-mail only"));
    let not_text = with(question("b"), "options", json!([1, 2]));

    for malformed in [missing, unknown, not_text] {
        assert_eq!(
            problem(&round(&[question("a"), malformed])).await,
            QuestionsProblem::MalformedQuestion { index: 1 }
        );
    }
}

#[tokio::test]
async fn ids_follow_the_item_list_key_rules() {
    let too_long = "q".repeat(33);
    for id in ["a b", "-a", "", "tâche", too_long.as_str()] {
        assert_eq!(
            problem(&round(&[question(id)])).await,
            QuestionsProblem::InvalidId { id: id.to_owned() },
            "{id:?}"
        );
    }
    assert_eq!(
        problem(&round(&[question("Nul")])).await,
        QuestionsProblem::ReservedId {
            id: "Nul".to_owned()
        }
    );
    assert_eq!(
        problem(&round(&[question("login"), question("LOGIN")])).await,
        QuestionsProblem::DuplicateId {
            id: "LOGIN".to_owned()
        }
    );
}

#[tokio::test]
async fn an_invalid_round_tells_the_agent_the_file_and_the_rule() {
    let document = round(&[with(question("login-id"), "why", json!(" "))]);

    let rejection = rejection(&document).await;

    assert_eq!(
        rejection.to_string(),
        "questions.json: question login-id has an empty why"
    );
}
