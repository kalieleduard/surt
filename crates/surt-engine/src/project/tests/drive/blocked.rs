use std::num::NonZeroU32;

use super::planned::{CYCLE_WORKFLOW, approved_with, review, work};
use crate::project::{Config, DriveOutcome, StepLabel};
use crate::runtime::Script;

const ITEMS: &[(&str, &[&str])] = &[("A", &[]), ("B", &["A"]), ("C", &[])];

fn two_iterations() -> Config {
    Config {
        max_iterations: NonZeroU32::new(2).expect("a positive limit"),
        ..Config::default()
    }
}

fn rejected_twice() -> Script {
    Script::new()
        .on("A/build", 1, 1, work("a1"))
        .on("A/review", 1, 1, review("changes_requested"))
        .on("A/build", 2, 1, work("a2"))
        .on("A/review", 2, 1, review("changes_requested"))
        .on("C/build", 1, 1, work("c1"))
        .on("C/review", 1, 1, review("approved"))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_loop_without_completion_blocks_its_item_and_holds_back_only_its_dependents() {
    let feature = approved_with(CYCLE_WORKFLOW, ITEMS, two_iterations()).await;

    let driven = feature.drive(rejected_twice()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked {
            steps: vec![StepLabel {
                key: "cycle".to_owned(),
                item: Some("A".to_owned()),
                iteration: None,
            }]
        }
    );
    let children: Vec<(String, String, i64)> = feature
        .dir
        .rows(
            "SELECT w.item_key, w.state, (SELECT count(*) FROM step_run s \
             WHERE s.workflow_run_id = w.id AND s.loop_step_run_id IS NOT NULL) \
             FROM workflow_run w WHERE w.parent_step_run_id IS NOT NULL ORDER BY w.item_key",
        )
        .await;
    assert_eq!(
        children,
        [
            ("A".to_owned(), "active".to_owned(), 4),
            ("B".to_owned(), "active".to_owned(), 0),
            ("C".to_owned(), "succeeded".to_owned(), 2),
        ]
    );
    let blocked: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT s.step_key, s.blocked_reason FROM step_run s WHERE s.state = 'blocked' \
             UNION ALL SELECT 'root', state FROM workflow_run WHERE parent_step_run_id IS NULL",
        )
        .await;
    assert_eq!(
        blocked,
        [
            ("cycle".to_owned(), "iterations_exhausted".to_owned()),
            ("root".to_owned(), "active".to_owned())
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rerunning_gives_a_blocked_loop_more_iterations_and_its_dependents_then_run() {
    let feature = approved_with(CYCLE_WORKFLOW, ITEMS, two_iterations()).await;
    feature
        .drive(rejected_twice())
        .await
        .outcome
        .expect("the first drive blocks");

    let resumed = feature
        .drive(
            Script::new()
                .on("A/build", 3, 1, work("a3"))
                .on("A/review", 3, 1, review("approved"))
                .on("B/build", 1, 1, work("b1"))
                .on("B/review", 1, 1, review("approved")),
        )
        .await;

    assert_eq!(
        resumed.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let loops: Vec<(String, String, i64)> = feature
        .dir
        .rows(
            "SELECT w.item_key, s.state, s.budget_base FROM step_run s \
             JOIN workflow_run w ON w.id = s.workflow_run_id WHERE s.step_key = 'cycle' \
             ORDER BY w.item_key",
        )
        .await;
    assert_eq!(
        loops,
        [
            ("A".to_owned(), "done".to_owned(), 2),
            ("B".to_owned(), "done".to_owned(), 0),
            ("C".to_owned(), "done".to_owned(), 0),
        ]
    );
}
