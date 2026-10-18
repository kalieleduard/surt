use serde_json::json;

use super::merging::{delivering_team, merged_feature};
use super::repository::commit_file;
use crate::project::{DriveOutcome, MergeIntoBase, Progress};

fn mentions_merge(progress: &Progress) -> bool {
    matches!(
        progress,
        Progress::StepStarted { step, .. } | Progress::StepFinished { step, .. }
            if step.key == "merge"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn never_leaves_the_base_branch_silently_and_records_the_merge_as_disabled() {
    let feature = merged_feature(MergeIntoBase::Never, Some("main")).await;
    feature.leave_main();
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(base));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "disabled" })]
    );
    assert!(
        !driven.progress.iter().any(mentions_merge),
        "{:?}",
        driven.progress
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_detached_base_is_not_merged_because_it_names_no_branch() {
    let feature = merged_feature(MergeIntoBase::FfOnly, None).await;
    feature.leave_main();
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(base));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "no_base_branch" })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_base_branch_that_moved_since_the_base_is_left_where_it_is() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
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

#[tokio::test(flavor = "multi_thread")]
async fn a_base_branch_checked_out_in_a_worktree_is_never_moved_underneath_it() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(base));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "worktree_not_clean" })]
    );
}
