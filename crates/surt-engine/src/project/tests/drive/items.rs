use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroU32;

use super::fixtures::Recording;
use super::planned::{Gauge, TASK_WORKFLOW, approved, approved_with, work};
use crate::project::{Config, DriveOutcome};
use crate::runtime::Script;

pub(super) fn limited(agents: u32) -> Config {
    Config {
        max_concurrent_agents: NonZeroU32::new(agents).expect("a positive limit"),
        ..Config::default()
    }
}

fn three_independent() -> Script {
    Script::new()
        .on("A/build", 1, 1, work("a"))
        .on("B/build", 1, 1, work("b"))
        .on("C/build", 1, 1, work("c"))
}

#[tokio::test(flavor = "multi_thread")]
async fn independent_items_run_in_parallel_up_to_the_agent_limit_each_holding_a_slot() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[]), ("C", &[])];
    let feature = approved_with(TASK_WORKFLOW, items, limited(2)).await;
    let runtime = Gauge::new(three_independent(), 2);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(runtime.peak(), 2);
    assert_eq!(runtime.started().len(), 3);
    let slots: Vec<(Option<i64>,)> = feature
        .dir
        .rows(
            "SELECT e.slot FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             WHERE s.step_key = 'build' ORDER BY e.id",
        )
        .await;
    let first_two: BTreeSet<Option<i64>> = slots.iter().take(2).map(|(slot,)| *slot).collect();
    assert_eq!(first_two, BTreeSet::from([Some(0), Some(1)]));
    assert!(matches!(slots[2], (Some(0 | 1),)), "{slots:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_limit_of_one_agent_runs_the_items_one_after_another_in_list_order() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[]), ("C", &[])];
    let feature = approved_with(TASK_WORKFLOW, items, limited(1)).await;
    let runtime = Gauge::new(three_independent(), 1);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(runtime.peak(), 1);
    assert_eq!(runtime.started(), ["A/build", "B/build", "C/build"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn each_item_runs_as_a_child_run_once_its_needs_succeeded() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &["A"]), ("C", &[])]).await;
    let runtime = Recording::new(
        Script::new()
            .on("A/build", 1, 1, work("a"))
            .on("B/build", 1, 1, work("b"))
            .on("C/build", 1, 1, work("c")),
    );

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let started: Vec<String> = runtime
        .requests()
        .iter()
        .map(|request| request.step.step.clone())
        .collect();
    assert_eq!(started.len(), 3, "{started:?}");
    assert_eq!(started[2], "B/build", "{started:?}");
    let children: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT item_key, state FROM workflow_run WHERE parent_step_run_id IS NOT NULL \
             ORDER BY item_key",
        )
        .await;
    assert_eq!(
        children,
        [
            ("A".to_owned(), "succeeded".to_owned()),
            ("B".to_owned(), "succeeded".to_owned()),
            ("C".to_owned(), "succeeded".to_owned()),
        ]
    );
    let implement: Vec<(String, bool)> = feature
        .dir
        .rows(
            "SELECT s.state, r.pinned_at IS NOT NULL FROM step_run s, artifact_revision r \
             JOIN artifact a ON a.id = r.artifact_id WHERE s.step_key = 'implement' AND a.name = 'plan'",
        )
        .await;
    assert_eq!(implement, [("done".to_owned(), true)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_child_reads_only_its_own_item_and_the_passed_parent_artifacts() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &[])]).await;
    let runtime = Recording::new(Script::new().on("A/build", 1, 1, work("a")).on(
        "B/build",
        1,
        1,
        work("b"),
    ));

    feature
        .drive_with(&runtime)
        .await
        .outcome
        .expect("drive returns");

    let requests = runtime.requests();
    let request = requests
        .iter()
        .find(|request| request.step.step == "A/build")
        .expect("A's Execution");
    let inputs = request
        .delivery_dir
        .parent()
        .expect("the Execution directory")
        .join("inputs");
    let item: serde_json::Value = serde_json::from_slice(
        &fs::read(inputs.join("item/item.json")).expect("the item index entry"),
    )
    .expect("JSON");
    assert_eq!(item["key"], "A");
    assert_eq!(item["title"], "Task A");
    assert_eq!(
        fs::read_to_string(inputs.join("item/tasks/A.md")).expect("A's task file"),
        "# Task A\n"
    );
    assert!(!inputs.join("item/tasks/B.md").exists());
    assert_eq!(
        fs::read_to_string(inputs.join("request/request.txt")).expect("the passed request"),
        "add login"
    );
    let items: Vec<(String, Option<String>)> = feature
        .dir
        .rows("SELECT input_name, item_key FROM execution_input WHERE input_name = 'item' ORDER BY item_key")
        .await;
    assert_eq!(
        items,
        [
            ("item".to_owned(), Some("A".to_owned())),
            ("item".to_owned(), Some("B".to_owned()))
        ]
    );
}
