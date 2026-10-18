use super::fixtures::{Feature, draft, label};
use crate::project::DriveOutcome;
use crate::runtime::{Behavior, Delivery, Script};

const RESETS_AT: i64 = 1_900_000_000;

fn crashes(script: Script, attempts: std::ops::RangeInclusive<u32>) -> Script {
    attempts.fold(script, |script, attempt| {
        script.on("write", 1, attempt, Behavior::Crash { exit_code: 1 })
    })
}

fn blocked_write() -> DriveOutcome {
    DriveOutcome::Blocked {
        steps: vec![label("write", Some(1))],
    }
}

async fn step_state(feature: &Feature) -> Vec<(String, Option<String>, i64)> {
    feature
        .dir
        .rows("SELECT state, blocked_reason, budget_base FROM step_run WHERE step_key = 'write'")
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn rerunning_unblocks_with_a_fresh_budget_while_attempts_keep_counting() {
    let feature = Feature::review().await;
    let first = feature.drive(crashes(Script::new(), 1..=3)).await;
    assert_eq!(first.outcome.expect("drive returns"), blocked_write());
    assert_eq!(
        step_state(&feature).await,
        [(
            "blocked".to_owned(),
            Some("attempts_exhausted".to_owned()),
            0
        )]
    );

    let second = feature.drive(crashes(Script::new(), 4..=6)).await;

    assert_eq!(second.outcome.expect("drive returns"), blocked_write());
    assert_eq!(
        step_state(&feature).await,
        [(
            "blocked".to_owned(),
            Some("attempts_exhausted".to_owned()),
            3
        )]
    );
    let third = feature
        .drive(Script::new().on("write", 1, 7, draft("# Draft\n")))
        .await;
    assert!(
        matches!(
            third.outcome.expect("drive returns"),
            DriveOutcome::AtGate { .. }
        ),
        "the seventh attempt delivers"
    );
    let unblocked: Vec<(i64,)> = feature
        .dir
        .rows("SELECT count(*) FROM event WHERE kind = 'step.unblocked'")
        .await;
    assert_eq!(unblocked, [(2,)]);
    let attempts: Vec<(i64, String)> = feature
        .dir
        .rows("SELECT attempt, state FROM execution ORDER BY attempt")
        .await;
    assert_eq!(attempts.len(), 7);
    assert_eq!(attempts[6], (7, "succeeded".to_owned()));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failure_retrying_cannot_fix_blocks_on_the_first_attempt() {
    let feature = Feature::review().await;
    let mismatch = Behavior::ModelMismatch {
        reported: "other-model".to_owned(),
        delivery: Delivery::new("done").output("draft", "draft.md", "# Draft\n"),
    };

    let driven = feature
        .drive(Script::new().on("write", 1, 1, mismatch))
        .await;

    assert_eq!(driven.outcome.expect("drive returns"), blocked_write());
    assert_eq!(
        step_state(&feature).await,
        [("blocked".to_owned(), Some("not_retryable".to_owned()), 0)]
    );
    let attempts: Vec<(String, Option<String>)> = feature
        .dir
        .rows("SELECT state, failure_class FROM execution")
        .await;
    assert_eq!(
        attempts,
        [("failed".to_owned(), Some("not_retryable".to_owned()))]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transient_failure_blocks_at_once_and_keeps_the_reset_time() {
    let feature = Feature::review().await;
    let limited = Behavior::LimitFailure {
        resets_at: Some(RESETS_AT),
    };

    let driven = feature
        .drive(Script::new().on("write", 1, 1, limited))
        .await;

    assert_eq!(driven.outcome.expect("drive returns"), blocked_write());
    assert_eq!(
        step_state(&feature).await,
        [("blocked".to_owned(), Some("not_retryable".to_owned()), 0)]
    );
    let attempts: Vec<(String, Option<String>, Option<i64>)> = feature
        .dir
        .rows(
            "SELECT state, failure_class, json_extract(runtime_info, '$.resets_at') FROM execution",
        )
        .await;
    assert_eq!(
        attempts,
        [(
            "failed".to_owned(),
            Some("transient".to_owned()),
            Some(RESETS_AT)
        )]
    );
}
