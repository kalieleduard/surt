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
