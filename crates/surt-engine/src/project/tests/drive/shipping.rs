use serde_json::json;

use super::super::fixtures::SUBJECT;
use super::fixtures::label;
use super::integration::{integration_branch, login, logout};
use super::team::{Team, assert_valid, builds, input, reviews, shipped, ships};
use crate::project::{Config, DriveOutcome, INTEGRATION_SCHEMA, StepLabel};
use crate::runtime::{Behavior, Delivery};

fn delivered_team() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on("B/build", 1, 1, builds(logout))
        .on("B/review", 1, 1, reviews("approved"))
        .on("ship", 1, 1, ships())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shipper_reads_the_manifest_and_the_snapshot_of_every_merged_task() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let team = delivered_team();

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    let shipper = team.request("ship", 1);
    let integration =
        input(&shipper, "integration", "integration.json").expect("the shipper reads it");
    assert_valid(INTEGRATION_SCHEMA, &integration);
    let mut tips = Vec::new();
    for key in ["A", "B"] {
        tips.push(feature.task_tip(key).await);
    }
    let item = |key: &str, reviewed: &str| {
        let merge = feature.merge_of(reviewed).expect("the merge").sha;
        json!({ "key": key, "reviewed_commit": reviewed, "merge_commit": merge })
    };
    assert_eq!(
        integration,
        json!({
            "branch": format!("surt/{SUBJECT}"),
            "base": { "ref": "main", "sha": feature.base() },
            "tip": tip,
            "items": [item("A", &tips[0]), item("B", &tips[1])],
        })
    );
    let plan = input(&shipper, "plan", "plan.json").expect("the shipper reads the manifest");
    assert_eq!(plan[0]["key"], "A");
    assert_eq!(shipper.launch.working_dir, feature.root());
    assert_eq!(feature.sha(&integration_branch()), Some(tip));
    let delivered: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT s.outcome FROM step_run s WHERE s.step_key IN ('snapshot', 'ship') \
             ORDER BY s.step_key",
        )
        .await;
    assert_eq!(delivered, [("done".to_owned(),), ("done".to_owned(),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shipper_never_runs_while_a_task_is_not_merged() {
    let feature = shipped(&[("A", &[]), ("B", &[])], Config::default()).await;
    let mismatch = Behavior::ModelMismatch {
        reported: "another-model".to_owned(),
        delivery: Delivery::new("done"),
    };
    let team = delivered_team().on("B/build", 1, 1, Box::new(move |_| mismatch.clone()));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked {
            steps: vec![StepLabel {
                item: Some("B".to_owned()),
                ..label("build", Some(1))
            }]
        }
    );
    assert!(
        team.requests()
            .iter()
            .all(|request| request.step.step != "ship")
    );
    let snapshots: Vec<(String,)> = feature
        .dir
        .rows("SELECT state FROM step_run WHERE step_key = 'snapshot'")
        .await;
    assert_eq!(snapshots, [("pending".to_owned(),)]);
}
