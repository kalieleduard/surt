use super::fixtures::{Execution, REPORT, valid, writer_contract};
use crate::artifacts::{AcceptedDelivery, ConsistencyCheck, DeliveryRejection};
use crate::runtime::{Behavior, Delivery, Verdict};

const MAX_APPROVED_COUNT: u64 = 5;

fn approval_counts_at_most_five(delivery: &AcceptedDelivery) -> Result<(), String> {
    let count = delivery.outputs["data"]
        .json
        .as_ref()
        .and_then(|data| data["count"].as_u64())
        .unwrap_or_default();
    if delivery.outcome == "approved" && count > MAX_APPROVED_COUNT {
        return Err(format!("approved with count {count}"));
    }
    Ok(())
}

fn checked_execution() -> Execution {
    Execution::with_contract(writer_contract().with_check(ConsistencyCheck {
        name: "bounded-approval",
        check: approval_counts_at_most_five,
    }))
}

fn with_count(outcome: &str, count: u64) -> Delivery {
    Delivery::new(outcome)
        .output("report", "report.md", REPORT)
        .output("data", "data.json", format!(r#"{{"count": {count}}}"#))
}

#[tokio::test]
async fn a_registered_consistency_check_rejects_an_inconsistent_delivery() {
    let rejection = checked_execution()
        .delivery_rejection(with_count("approved", 9))
        .await;

    assert_eq!(
        rejection,
        DeliveryRejection::Inconsistent {
            check: "bounded-approval".to_owned(),
            detail: "approved with count 9".to_owned()
        }
    );
}

#[tokio::test]
async fn a_registered_consistency_check_lets_a_consistent_delivery_through() {
    let execution = checked_execution();

    let changes = execution
        .run(Behavior::Deliver(with_count("changes_requested", 9)))
        .await
        .expect("the fake completes");
    let approval = execution
        .run(Behavior::Deliver(valid("approved")))
        .await
        .expect("the fake completes");

    assert!(matches!(changes, Verdict::Accepted(_)), "{changes:?}");
    assert!(matches!(approval, Verdict::Accepted(_)), "{approval:?}");
}
