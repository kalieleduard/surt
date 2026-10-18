use std::fs;

use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::integration::integration_branch;
use super::merging::{delivering_team, merged_feature};
use super::repository::commit_file;
use crate::project::{BaseMerge, MergeIntoBase, NotMerged, RunSelector, StatusScope};

impl Feature {
    async fn status_merge(&self) -> Option<BaseMerge> {
        let statuses = self
            .project
            .status(StatusScope::Feature(SUBJECT, RunSelector::Current))
            .await
            .expect("status reads");
        statuses[0].merge.clone()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_no_merge_result_before_the_merge_step_ends() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;

    assert_eq!(feature.status_merge().await, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_the_fast_forward_it_recorded() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    feature.leave_main();
    let base = feature.base();

    feature.drive_with(&delivering_team()).await;

    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert_eq!(
        feature.status_merge().await,
        Some(BaseMerge::Merged {
            from: base,
            to: tip
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_where_the_moved_base_branch_was_found() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    let moved = commit_file(&feature.root(), "NEWS.md", "news\n");
    feature.leave_main();

    feature.drive_with(&delivering_team()).await;

    assert_eq!(
        feature.status_merge().await,
        Some(BaseMerge::Skipped(NotMerged::BaseMoved {
            found: Some(moved)
        }))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_a_detached_base_as_no_base_branch() {
    let feature = merged_feature(MergeIntoBase::FfOnly, None).await;
    feature.leave_main();

    feature.drive_with(&delivering_team()).await;

    assert_eq!(
        feature.status_merge().await,
        Some(BaseMerge::Skipped(NotMerged::NoBaseBranch))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_a_base_branch_checked_out_in_a_dirty_worktree() {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    fs::write(feature.root().join("README.md"), "edited\n").expect("the edit is written");

    feature.drive_with(&delivering_team()).await;

    assert_eq!(
        feature.status_merge().await,
        Some(BaseMerge::Skipped(NotMerged::WorktreeNotClean))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn status_reports_a_disabled_merge_as_recorded() {
    let feature = merged_feature(MergeIntoBase::Never, Some("main")).await;
    feature.leave_main();

    feature.drive_with(&delivering_team()).await;

    assert_eq!(
        feature.status_merge().await,
        Some(BaseMerge::Skipped(NotMerged::Disabled))
    );
}
