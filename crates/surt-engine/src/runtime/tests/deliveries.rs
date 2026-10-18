use super::fixtures::{DATA, Execution, REPORT, rejection, valid};
use crate::artifacts::DeliveryRejection;
use crate::runtime::{Behavior, Delivery, Verdict};

#[tokio::test]
async fn a_valid_delivery_is_accepted_with_its_outcome_and_the_bytes_it_wrote() {
    let execution = Execution::new();

    let verdict = execution
        .run(Behavior::Deliver(valid("approved")))
        .await
        .expect("the fake completes");

    let Verdict::Accepted(delivery) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    assert_eq!(delivery.outcome, "approved");
    let report = &delivery.outputs["report"].files;
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].path, "report.md");
    assert_eq!(report[0].bytes, REPORT.as_bytes());
    let data = &delivery.outputs["data"].files;
    assert_eq!(data[0].path, "out/data.json");
    assert_eq!(data[0].bytes, DATA.as_bytes());
}

#[tokio::test]
async fn changes_requested_with_valid_files_is_a_successful_outcome() {
    let verdict = Execution::new()
        .run(Behavior::Deliver(valid("changes_requested")))
        .await
        .expect("the fake completes");

    let Verdict::Accepted(delivery) = verdict else {
        panic!("expected an accepted delivery, got {verdict:?}");
    };
    assert_eq!(delivery.outcome, "changes_requested");
}

#[tokio::test]
async fn files_without_a_final_json_are_rejected() {
    let rejection = rejection(valid("approved").final_json(None)).await;

    assert_eq!(rejection, DeliveryRejection::MissingEnvelope);
}

#[tokio::test]
async fn a_final_json_that_does_not_parse_is_rejected() {
    let truncated = r#"{"outcome": "approved""#;

    let rejection = rejection(valid("approved").final_json(Some(truncated))).await;

    assert_eq!(rejection, DeliveryRejection::InvalidEnvelope);
}

#[tokio::test]
async fn an_envelope_with_fields_beyond_outcome_and_outputs_is_rejected() {
    let envelope = r#"{"outcome": "approved", "outputs": {"report": "report.md", "data": "out/data.json"}, "summary": "done"}"#;

    let rejection = rejection(valid("approved").final_json(Some(envelope))).await;

    assert_eq!(rejection, DeliveryRejection::InvalidEnvelope);
}

#[tokio::test]
async fn an_output_path_that_is_not_a_string_is_rejected() {
    let envelope = r#"{"outcome": "approved", "outputs": {"report": "report.md", "data": 7}}"#;

    let rejection = rejection(valid("approved").final_json(Some(envelope))).await;

    assert_eq!(rejection, DeliveryRejection::InvalidEnvelope);
}

#[tokio::test]
async fn an_outcome_outside_the_contract_is_rejected() {
    let rejection = rejection(valid("done")).await;

    assert_eq!(
        rejection,
        DeliveryRejection::UnknownOutcome {
            outcome: "done".to_owned()
        }
    );
}

#[tokio::test]
async fn a_declared_output_missing_from_the_envelope_is_rejected() {
    let delivery = Delivery::new("approved").output("report", "report.md", REPORT);

    let rejection = rejection(delivery).await;

    assert_eq!(
        rejection,
        DeliveryRejection::MissingOutput {
            name: "data".to_owned()
        }
    );
}

#[tokio::test]
async fn an_output_the_contract_does_not_declare_is_rejected() {
    let delivery = valid("approved").output("notes", "notes.md", "extra");

    let rejection = rejection(delivery).await;

    assert_eq!(
        rejection,
        DeliveryRejection::UndeclaredOutput {
            name: "notes".to_owned()
        }
    );
}
