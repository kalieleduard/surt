use serde_json::json;

use super::fault_intentions::CrashingAfter;
use super::faults::assert_crashed;
use super::fixtures::{Feature, label};
use super::integration::integration_branch;
use super::merging::{delivering_team, merged_feature};
use super::repository::commit_file;
use crate::project::{Checkpoint, DriveOutcome, MergeIntoBase, Progress};

async fn crashed_at_the_intention() -> (Feature, String, String) {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    feature.leave_main();
    let base = feature.base();
    let crashing = CrashingAfter {
        team: delivering_team(),
        step: "ship",
        checkpoint: Checkpoint::Intention,
        project: &feature.project,
    };

    assert_crashed(
        feature.drive_with(&crashing).await.outcome,
        Checkpoint::Intention,
    );

    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert_eq!(feature.sha("main"), Some(base.clone()));
    (feature, base, tip)
}

impl Feature {
    async fn merge_attempts(&self) -> Vec<(i64, String)> {
        self.dir
            .rows(
                "SELECT e.attempt, e.state FROM execution e JOIN step_run s ON s.id = e.step_run_id \
                 WHERE s.step_key = 'merge' ORDER BY e.attempt",
            )
            .await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_before_the_base_branch_moved_runs_the_fast_forward_again() {
    let (feature, base, tip) = crashed_at_the_intention().await;

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
    assert_eq!(
        feature.merge_attempts().await,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
    let discarded = feature.dir.event_data("intention.discarded").await;
    assert_eq!(
        discarded,
        [
            json!({ "intention": "ref_move", "ref": "refs/heads/main", "old_sha": base, "new_sha": tip })
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_after_the_base_branch_moved_records_the_merge_without_moving_it_again() {
    let (feature, base, tip) = crashed_at_the_intention().await;
    feature.git(&["update-ref", "refs/heads/main", &tip]);

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert!(
        driven.progress.contains(&Progress::IntegrationRecorded {
            step: label("merge", None),
            branch: "main".to_owned(),
            merge: tip.clone(),
        }),
        "{:?}",
        driven.progress
    );
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
    assert_eq!(feature.sha("main"), Some(tip));
    assert_eq!(feature.dir.event_data("intention.confirmed").await.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_base_branch_moved_by_someone_else_during_the_crash_is_not_merged() {
    let (feature, _, _) = crashed_at_the_intention().await;
    feature.git(&["checkout", "--quiet", "main"]);
    let moved = commit_file(&feature.root(), "NEWS.md", "news\n");
    feature.leave_main();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(moved.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "base_moved", "found": moved })]
    );
}
