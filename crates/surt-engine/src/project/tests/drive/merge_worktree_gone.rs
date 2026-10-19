use serde_json::json;

use super::merge_interrupted::{Interrupted, interrupted};
use super::merging::delivering_team;
use crate::project::DriveOutcome;

#[tokio::test(flavor = "multi_thread")]
async fn an_interrupted_fast_forward_whose_worktree_is_gone_is_decided_again() {
    let Interrupted { feature, base, tip } = interrupted().await;
    feature.git(&["reset", "--hard", "--quiet", &base]);
    let gone = feature.root().join("removed-worktree");
    feature
        .dir
        .execute(&format!(
            "UPDATE ref_move SET worktree_path = '{}'",
            gone.display()
        ))
        .await;

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
    assert_eq!(feature.kept_moves().await, []);
}
