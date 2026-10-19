use serde_json::json;

use super::super::fixtures::SUBJECT;
use super::fixtures::Feature;
use super::integration::integration_branch;
use super::merging::{delivering_team, merged_catalog, merging, pre_merge_catalog};
use super::team::approved;
use crate::project::{DriveOutcome, MergeIntoBase, RunSelector, StatusScope};

async fn chosen_feature(file: MergeIntoBase, chosen: MergeIntoBase) -> Feature {
    let feature = Feature::on_base_choosing(
        merged_catalog(),
        "merged",
        merging(file),
        Some("main"),
        Some(chosen),
    )
    .await;
    approved(feature, &[("A", &[])]).await
}

impl Feature {
    async fn reopened_with(mut self, file: MergeIntoBase) -> Feature {
        self.project = self.dir.project(merged_catalog(), merging(file)).await;
        self
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chosen_ff_only_frozen_with_the_run_merges_although_the_project_file_says_never() {
    let feature = chosen_feature(MergeIntoBase::Never, MergeIntoBase::FfOnly).await;
    feature.leave_main();
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chosen_never_frozen_with_the_run_wins_over_the_project_file_however_it_is_edited() {
    let feature = chosen_feature(MergeIntoBase::FfOnly, MergeIntoBase::Never)
        .await
        .reopened_with(MergeIntoBase::FfOnly)
        .await;
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
}

#[tokio::test(flavor = "multi_thread")]
async fn a_run_frozen_without_the_merge_step_finishes_without_merging_or_a_merge_result() {
    let feature = Feature::on_base(
        pre_merge_catalog(),
        "merged",
        merging(MergeIntoBase::FfOnly),
        Some("main"),
    )
    .await;
    let feature = approved(feature, &[("A", &[])])
        .await
        .reopened_with(MergeIntoBase::FfOnly)
        .await;
    feature.leave_main();
    let base = feature.base();

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(base));
    assert!(feature.merge_done().await.is_empty());
    assert_eq!(
        feature.project.base_merge(&feature.run).await.ok(),
        Some(None)
    );
    let statuses = feature
        .project
        .status(StatusScope::Feature(SUBJECT, RunSelector::Current))
        .await
        .expect("status reads");
    assert_eq!(statuses[0].merge, None);
}
