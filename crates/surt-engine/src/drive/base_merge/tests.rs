use serde_json::json;

use super::{BaseMerge, NotMerged};

#[test]
fn a_skip_records_its_reason_beside_the_outcome() {
    let moved = BaseMerge::Skipped(NotMerged::BaseMoved {
        found: Some("b22e310".to_owned()),
    });

    assert_eq!(
        moved.data(),
        json!({ "outcome": "skipped", "reason": "base_moved", "found": "b22e310" })
    );
    assert_eq!(
        BaseMerge::Skipped(NotMerged::Disabled).data(),
        json!({ "outcome": "skipped", "reason": "disabled" })
    );
}

#[test]
fn recorded_data_reads_back_and_an_older_event_reads_as_nothing() {
    let data = r#"{"outcome": "merged", "from": "a11c9d0", "to": "4e1b2c0"}"#;

    assert_eq!(
        BaseMerge::from_data(data),
        Some(BaseMerge::Merged {
            from: "a11c9d0".to_owned(),
            to: "4e1b2c0".to_owned()
        })
    );
    assert_eq!(BaseMerge::from_data(r#"{"outcome": "done"}"#), None);
}
