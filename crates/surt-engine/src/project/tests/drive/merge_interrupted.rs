use std::fs;

use serde_json::json;

use super::fixtures::{Feature, label};
use super::integration::integration_branch;
use super::merging::{delivering_team, merged_feature};
use crate::project::{DriveOutcome, MergeIntoBase};

const REFUSING_MAIN: &str =
    "#!/bin/sh\n[ \"$1\" = prepared ] || exit 0\n! grep -q ' refs/heads/main$'\n";
const HOOK: &str = "reference-transaction";

struct Interrupted {
    feature: Feature,
    base: String,
    tip: String,
}

async fn interrupted() -> Interrupted {
    let feature = merged_feature(MergeIntoBase::FfOnly, Some("main")).await;
    let base = feature.base();
    feature.hook(HOOK, REFUSING_MAIN);
    feature.blocks().await;
    feature.unhook(HOOK);
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    Interrupted { feature, base, tip }
}

impl Feature {
    async fn blocks(&self) {
        let driven = self.drive_with(&delivering_team()).await;
        assert_eq!(
            driven.outcome.expect("drive returns"),
            DriveOutcome::Blocked {
                steps: vec![label("merge", None)]
            }
        );
    }

    async fn merge_failures(&self) -> Vec<(i64, Option<String>)> {
        self.dir
            .rows(
                "SELECT e.attempt, e.error_code FROM execution e \
                 JOIN step_run s ON s.id = e.step_run_id \
                 WHERE s.step_key = 'merge' ORDER BY e.attempt",
            )
            .await
    }

    async fn kept_moves(&self) -> Vec<(String, String)> {
        self.dir
            .rows("SELECT ref_name, old_sha FROM ref_move")
            .await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_fast_forward_that_stopped_between_the_files_and_the_ref_blocks_the_step() {
    let Interrupted {
        feature,
        base,
        tip: _,
    } = interrupted().await;

    assert_eq!(feature.sha("main"), Some(base.clone()));
    assert_eq!(
        feature.merge_failures().await,
        [
            (1, Some("git_failed".to_owned())),
            (2, Some("merge_into_base_interrupted".to_owned()))
        ]
    );
    assert_eq!(
        feature.kept_moves().await,
        [("refs/heads/main".to_owned(), base)]
    );
    let blocked = feature
        .project
        .blocked_steps(&feature.run)
        .await
        .expect("blocked steps");
    let failure = blocked
        .first()
        .and_then(|step| step.last_failure.as_ref())
        .expect("the merge step failed");
    assert!(
        failure
            .message
            .starts_with("merge into main was interrupted; "),
        "{}",
        failure.message
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn finishing_the_interrupted_fast_forward_by_hand_records_the_merge() {
    let Interrupted { feature, base, tip } = interrupted().await;
    feature.git(&["merge", "--ff-only", "--quiet", &tip]);

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

#[tokio::test(flavor = "multi_thread")]
async fn undoing_the_interrupted_fast_forward_lets_it_run_again() {
    let Interrupted { feature, base, tip } = interrupted().await;
    feature.git(&["reset", "--hard", "--quiet", &base]);

    let driven = feature.drive_with(&delivering_team()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(
        fs::read_to_string(feature.root().join("login.txt")).ok(),
        Some("login\n".to_owned())
    );
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn leaving_the_interrupted_fast_forward_as_it_is_blocks_again() {
    let Interrupted {
        feature,
        base,
        tip: _,
    } = interrupted().await;

    feature.blocks().await;

    assert_eq!(feature.sha("main"), Some(base.clone()));
    assert_eq!(
        feature.kept_moves().await,
        [("refs/heads/main".to_owned(), base)]
    );
}
