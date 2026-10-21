use std::fs;

use serde_json::{Value, json};

use super::super::fixtures::SUBJECT;
use super::fault_intentions::CrashingAfter;
use super::faults::assert_crashed;
use super::fixtures::Feature;
use super::merging::{delivering_team, merged_feature};
use super::repository::commit_file;
use crate::project::{Checkpoint, MergeIntoBase};

const REFUSING_MAIN: &str =
    "#!/bin/sh\n[ \"$1\" = prepared ] || exit 0\n! grep -q ' refs/heads/main$'\n";
const HOOK: &str = "reference-transaction";

struct Advanced {
    feature: Feature,
    start: String,
    moved: String,
}

async fn advanced(mode: MergeIntoBase, checked_out: bool) -> Advanced {
    let feature = merged_feature(mode, Some("main")).await;
    let start = feature.base();
    let moved = commit_file(&feature.root(), "NEWS.md", "news\n");
    if !checked_out {
        feature.leave_main();
    }
    Advanced {
        feature,
        start,
        moved,
    }
}

async fn conflicting(checked_out: bool) -> (Feature, String) {
    let feature = merged_feature(MergeIntoBase::Merge, Some("main")).await;
    let moved = commit_file(&feature.root(), "login.txt", "mine\n");
    if !checked_out {
        feature.leave_main();
    }
    (feature, moved)
}

impl Feature {
    fn parents(&self, commit: &str) -> Vec<String> {
        let listed = self.git(&["rev-list", "--parents", "-n", "1", commit]);
        listed
            .split_whitespace()
            .skip(1)
            .map(str::to_owned)
            .collect()
    }

    fn merge_commits_over(&self, base: &str) -> usize {
        let range = format!("{base}..main");
        self.git(&["rev-list", "--merges", "--first-parent", &range])
            .lines()
            .count()
    }

    async fn kept_move_target(&self) -> String {
        let rows: Vec<(String,)> = self.dir.rows("SELECT new_sha FROM ref_move").await;
        rows.into_iter().next().expect("a kept ref move").0
    }
}

fn assert_not_clean(done: &[Value]) {
    let [only] = done else {
        panic!("one merge result expected: {done:?}");
    };
    assert_eq!(only["outcome"], "skipped", "{only}");
    assert_eq!(only["reason"], "worktree_not_clean", "{only}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_base_still_at_the_start_commit_is_fast_forwarded_in_merge_mode() {
    let feature = merged_feature(MergeIntoBase::Merge, Some("main")).await;
    feature.leave_main();
    let base = feature.base();

    feature.succeeds().await;

    let tip = feature.tip();
    assert_eq!(feature.sha("main"), Some(tip.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": base, "to": tip })]
    );
    assert_eq!(
        feature.git(&["reflog", "-1", "--format=%gs", "main"]),
        format!("surt: fast-forward main to surt/{SUBJECT}")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_advanced_base_not_checked_out_gets_a_merge_commit_of_the_base_and_the_run_tip() {
    let Advanced {
        feature,
        start,
        moved,
    } = advanced(MergeIntoBase::Merge, false).await;

    feature.succeeds().await;

    let tip = feature.tip();
    let merge = feature.sha("main").expect("main exists");
    assert_eq!(feature.parents(&merge), [moved.clone(), tip]);
    assert!(
        feature
            .git(&["log", "-1", "--format=%B", &merge])
            .contains("Surt-Run: ")
    );
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": moved, "to": merge, "merge_commit": true })]
    );
    assert_eq!(feature.merge_commits_over(&start), 1);
    assert_eq!(
        feature.git(&["reflog", "-1", "--format=%gs", "main"]),
        format!("surt: merge surt/{SUBJECT} into main")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_advanced_base_checked_out_clean_gets_the_merge_commit_and_both_sides_files() {
    let Advanced { feature, moved, .. } = advanced(MergeIntoBase::Merge, true).await;

    feature.succeeds().await;

    let tip = feature.tip();
    let merge = feature.sha("main").expect("main exists");
    assert_eq!(feature.parents(&merge), [moved.clone(), tip]);
    assert_eq!(feature.sha("HEAD"), Some(merge.clone()));
    assert_eq!(feature.file("NEWS.md").as_deref(), Some("news\n"));
    assert_eq!(feature.file("login.txt").as_deref(), Some("login\n"));
    assert_eq!(feature.git(&["status", "--porcelain"]), "");
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": moved, "to": merge, "merge_commit": true })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_advanced_base_is_left_alone_and_the_conflict_is_recorded() {
    let (feature, moved) = conflicting(false).await;

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(moved));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "conflict", "paths": ["login.txt"] })]
    );
    assert!(feature.kept_moves().await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_conflicting_advanced_base_checked_out_keeps_its_files_and_branch() {
    let (feature, moved) = conflicting(true).await;

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(moved.clone()));
    assert_eq!(feature.sha("HEAD"), Some(moved));
    assert_eq!(feature.file("login.txt").as_deref(), Some("mine\n"));
    assert_eq!(feature.git(&["status", "--porcelain"]), "");
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "conflict", "paths": ["login.txt"] })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dirty_checkout_of_an_unmoved_base_is_left_alone_in_merge_mode() {
    let feature = merged_feature(MergeIntoBase::Merge, Some("main")).await;
    let base = feature.base();
    fs::write(feature.root().join("README.md"), "edited\n").expect("an edit");

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(base));
    assert_eq!(feature.file("README.md").as_deref(), Some("edited\n"));
    assert_eq!(feature.file("login.txt"), None);
    assert_not_clean(&feature.merge_done().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dirty_checkout_of_an_advanced_base_is_left_alone_in_merge_mode() {
    let Advanced { feature, moved, .. } = advanced(MergeIntoBase::Merge, true).await;
    fs::write(feature.root().join("README.md"), "edited\n").expect("an edit");

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(moved));
    assert_eq!(feature.file("README.md").as_deref(), Some("edited\n"));
    assert_eq!(feature.file("login.txt"), None);
    assert_not_clean(&feature.merge_done().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dirty_checkout_wins_over_a_conflict() {
    let (feature, moved) = conflicting(true).await;
    fs::write(feature.root().join("README.md"), "edited\n").expect("an edit");

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(moved));
    assert_not_clean(&feature.merge_done().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn ff_only_still_skips_with_base_moved_when_the_base_advanced() {
    let Advanced { feature, moved, .. } = advanced(MergeIntoBase::FfOnly, false).await;

    feature.succeeds().await;

    assert_eq!(feature.sha("main"), Some(moved.clone()));
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "skipped", "reason": "base_moved", "found": moved })]
    );
}

async fn crashed_at_the_intention(checked_out: bool) -> (Advanced, String) {
    let advanced = advanced(MergeIntoBase::Merge, checked_out).await;
    let crashing = CrashingAfter {
        team: delivering_team(),
        step: "ship",
        checkpoint: Checkpoint::Intention,
        project: &advanced.feature.project,
    };
    assert_crashed(
        advanced.feature.drive_with(&crashing).await.outcome,
        Checkpoint::Intention,
    );
    assert_eq!(advanced.feature.sha("main"), Some(advanced.moved.clone()));
    let intended = advanced.feature.kept_move_target().await;
    (advanced, intended)
}

async fn resumed_once(advanced: &Advanced, merge: &str) {
    let feature = &advanced.feature;
    feature.succeeds().await;
    assert_eq!(feature.sha("main").as_deref(), Some(merge));
    assert_eq!(
        feature.parents(merge),
        [advanced.moved.clone(), feature.tip()]
    );
    assert_eq!(feature.merge_commits_over(&advanced.start), 1);
    assert_eq!(
        feature.merge_done().await,
        [json!({ "outcome": "merged", "from": advanced.moved, "to": merge, "merge_commit": true })]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_before_the_base_moved_redoes_the_merge_commit_with_the_same_id() {
    let (advanced, intended) = crashed_at_the_intention(false).await;

    resumed_once(&advanced, &intended).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_after_the_base_moved_records_the_merge_commit_without_another_one() {
    let (advanced, intended) = crashed_at_the_intention(false).await;
    advanced
        .feature
        .git(&["update-ref", "refs/heads/main", &intended]);

    resumed_once(&advanced, &intended).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_before_the_checked_out_base_moved_redoes_the_merge_commit_with_the_same_id() {
    let (advanced, intended) = crashed_at_the_intention(true).await;

    resumed_once(&advanced, &intended).await;

    assert_eq!(
        advanced.feature.file("login.txt").as_deref(),
        Some("login\n")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_crash_after_the_checked_out_base_moved_records_the_merge_commit_without_another_one() {
    let (advanced, intended) = crashed_at_the_intention(true).await;
    advanced
        .feature
        .git(&["merge", "--ff-only", "--quiet", &intended]);

    resumed_once(&advanced, &intended).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_merge_commit_move_that_stopped_before_the_ref_runs_again_once_undone() {
    let advanced = advanced(MergeIntoBase::Merge, true).await;
    advanced.feature.hook(HOOK, REFUSING_MAIN);
    advanced.feature.drive_with(&delivering_team()).await;
    advanced.feature.unhook(HOOK);
    assert_eq!(advanced.feature.sha("main"), Some(advanced.moved.clone()));
    let intended = advanced.feature.kept_move_target().await;
    advanced
        .feature
        .git(&["reset", "--hard", "--quiet", &advanced.moved]);

    resumed_once(&advanced, &intended).await;
}
