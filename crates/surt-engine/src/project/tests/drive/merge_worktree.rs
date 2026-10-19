use std::fs;
use std::path::Path;

use serde_json::json;

use super::fixtures::Feature;
use super::integration::integration_branch;
use super::merging::{delivering_team, merged_feature};
use crate::project::{DriveOutcome, MergeIntoBase};

async fn checked_out_feature() -> (Feature, String) {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    let base = feature.base();
    (feature, base)
}

impl Feature {
    async fn succeeds(&self) {
        let driven = self.drive_with(&delivering_team()).await;
        assert_eq!(
            driven.outcome.expect("drive returns"),
            DriveOutcome::Succeeded
        );
    }

    fn tip(&self) -> String {
        self.sha(&integration_branch())
            .expect("the integration branch")
    }

    fn file(&self, name: &str) -> Option<String> {
        fs::read_to_string(self.root().join(name)).ok()
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.canonicalize().ok() == right.canonicalize().ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_clean_checked_out_base_branch_moves_together_with_its_files() {
    let (feature, base) = checked_out_feature().await;

    feature.succeeds().await;

    let tip = feature.tip();
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(feature.sha("HEAD"), Some(tip.clone()));
    assert_eq!(feature.file("login.txt").as_deref(), Some("login\n"));
    assert_eq!(feature.git(&["status", "--porcelain"]), "");
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
    let intended = feature.dir.event_data("ref_move.intended").await;
    let worktree = intended
        .last()
        .and_then(|data| data["worktree_path"].as_str())
        .map(Path::new)
        .expect("the intention names the worktree");
    assert!(same_path(worktree, &feature.root()), "{intended:?}");
}

async fn left_alone(feature: &Feature, base: &str) {
    feature.succeeds().await;
    assert_eq!(feature.sha("main"), Some(base.to_owned()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "worktree_not_clean" })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tracked_change_in_the_checked_out_worktree_leaves_the_base_branch_alone() {
    let (feature, base) = checked_out_feature().await;
    fs::write(feature.root().join("README.md"), "edited\n").expect("an edit");

    left_alone(&feature, &base).await;

    assert_eq!(feature.file("README.md").as_deref(), Some("edited\n"));
    assert_eq!(feature.file("login.txt"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_merge_in_progress_in_the_checked_out_worktree_leaves_the_base_branch_alone() {
    let (feature, base) = checked_out_feature().await;
    let marker = feature.root().join(".git").join("MERGE_HEAD");
    fs::write(&marker, format!("{base}\n")).expect("a merge marker");

    left_alone(&feature, &base).await;

    assert!(marker.exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_untracked_file_the_fast_forward_does_not_touch_is_left_alone_and_merged_around() {
    let (feature, base) = checked_out_feature().await;
    fs::write(feature.root().join("scratch.txt"), "mine\n").expect("a scratch file");

    feature.succeeds().await;

    let tip = feature.tip();
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(feature.file("scratch.txt").as_deref(), Some("mine\n"));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_untracked_file_the_fast_forward_would_overwrite_leaves_the_base_branch_alone() {
    let (feature, base) = checked_out_feature().await;
    fs::write(feature.root().join("login.txt"), "mine\n").expect("an untracked file");

    left_alone(&feature, &base).await;

    assert_eq!(feature.file("login.txt").as_deref(), Some("mine\n"));
    assert_eq!(feature.dir.event_data("intention.discarded").await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_post_merge_hook_runs_and_its_failure_does_not_undo_the_merge() {
    let (feature, base) = checked_out_feature().await;
    feature.hook("post-merge", "#!/bin/sh\necho ran > hook-ran.txt\nexit 1\n");

    feature.succeeds().await;

    let tip = feature.tip();
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(feature.file("hook-ran.txt").as_deref(), Some("ran\n"));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
}
