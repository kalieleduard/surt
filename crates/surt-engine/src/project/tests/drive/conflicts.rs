use std::fs;
use std::path::Path;

use serde_json::json;

use super::super::fixtures::SUBJECT;
#[cfg(unix)]
use super::checks::{check, with_checks};
use super::fixtures::Feature;
use super::integration::integration_branch;
#[cfg(unix)]
use super::integration::{login, logout};
use super::repository::{commit_file, git, git_succeeds};
use super::team::{
    Act, INTEGRATED_TASK, Team, assert_valid, builds, built, failures, input, reviews,
};
use crate::project::{CONFLICT_SCHEMA, Config, Conflict, DriveOutcome, ItemState, StatusScope};

const KEYS: [&str; 2] = ["A", "B"];

fn shared_login(worktree: &Path) -> String {
    commit_file(worktree, "shared.txt", "login\n")
}

fn shared_logout(worktree: &Path) -> String {
    commit_file(worktree, "shared.txt", "logout\n")
}

fn without_merging(worktree: &Path) -> String {
    commit_file(worktree, "shared.txt", "login\nlogout\n")
}

fn merging(worktree: &Path) -> String {
    let tip = format!("surt/{SUBJECT}");
    let _conflicted = git_succeeds(worktree, &["merge", "--quiet", "--no-edit", &tip]);
    fs::write(worktree.join("shared.txt"), "login\nlogout\n").expect("the resolution");
    git(worktree, &["add", "shared.txt"]);
    git(worktree, &["commit", "--quiet", "--no-edit"]);
    git(worktree, &["rev-parse", "HEAD"])
}

fn rivals(first_iteration: [Act; 2], second_iteration: &[Act]) -> Team {
    KEYS.into_iter()
        .zip(first_iteration)
        .fold(Team::new(), |team, (key, act)| {
            let team = team
                .on(&format!("{key}/build"), 1, 1, builds(act))
                .on(&format!("{key}/review"), 1, 1, reviews("approved"))
                .on(&format!("{key}/review"), 2, 1, reviews("approved"));
            second_iteration
                .iter()
                .zip(1..)
                .fold(team, |team, (act, attempt)| {
                    team.on(&format!("{key}/build"), 2, attempt, builds(*act))
                })
        })
}

struct Conflicted {
    key: &'static str,
    merged_first: &'static str,
}

async fn conflicted(feature: &Feature) -> Conflicted {
    let integrations = feature.integrations().await;
    let key = KEYS
        .into_iter()
        .find(|key| {
            integrations
                .iter()
                .any(|(item, _, outcome)| item == key && outcome == "conflict")
        })
        .expect("one task conflicted");
    let merged_first = KEYS
        .into_iter()
        .find(|other| *other != key)
        .expect("the other task");
    assert_eq!(
        integrations
            .iter()
            .map(|(item, iteration, outcome)| (item == key, *iteration, outcome.as_str()))
            .collect::<Vec<_>>(),
        if key == "A" {
            vec![
                (true, 1, "conflict"),
                (true, 2, "merged"),
                (false, 1, "merged"),
            ]
        } else {
            vec![
                (false, 1, "merged"),
                (true, 1, "conflict"),
                (true, 2, "merged"),
            ]
        }
    );
    Conflicted { key, merged_first }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_merge_conflict_sends_the_task_back_to_its_builder_with_a_conflict() {
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        Config::default(),
    )
    .await;
    let team = rivals([shared_login, shared_logout], &[merging]);

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let Conflicted { key, merged_first } = conflicted(&feature).await;
    let first = feature.task_tip(merged_first).await;
    let first_merge = feature.merge_of(&first).expect("the first merge");
    let builder = team.request(&format!("{key}/build"), 2);
    let conflict = input(&builder, "conflict", "conflict.json").expect("the builder reads it");
    assert_valid(CONFLICT_SCHEMA, &conflict);
    let resolved = feature.task_tip(key).await;
    let rejected = feature.git(&["rev-parse", &format!("{resolved}^1")]);
    assert_eq!(
        conflict,
        json!({
            "kind": "merge",
            "integration_tip": first_merge.sha,
            "reviewed_commit": rejected,
            "paths": ["shared.txt"],
        })
    );
    let reviewer = team.request(&format!("{key}/review"), 2);
    assert_eq!(
        input(&reviewer, "conflict", "conflict.json"),
        Some(conflict)
    );
    let merge = feature
        .merge_of(&resolved)
        .expect("the resolution is merged");
    assert_eq!(merge.first_parent, first_merge.sha);
    assert_eq!(feature.sha(&integration_branch()), Some(merge.sha.clone()));
    assert_eq!(
        feature.git(&["show", &format!("{}:shared.txt", merge.sha)]),
        "login\nlogout"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn after_a_conflict_the_builders_commit_must_descend_from_the_integration_tip() {
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        Config::default(),
    )
    .await;
    let team = rivals([shared_login, shared_logout], &[without_merging, merging]);

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let Conflicted { merged_first, .. } = conflicted(&feature).await;
    let first = feature.task_tip(merged_first).await;
    let tip = feature.merge_of(&first).expect("the first merge").sha;
    let refused = failures(&feature, "build").await;
    assert_eq!(refused.len(), 1, "{refused:?}");
    let (iteration, attempt, code, message) = &refused[0];
    assert_eq!(
        (*iteration, *attempt, code.as_str()),
        (2, 1, "invalid_delivery")
    );
    assert!(
        message.ends_with(&format!(
            " does not descend from the conflict's integration tip {tip}"
        )),
        "{message}"
    );
}

#[cfg(unix)]
fn unify(worktree: &Path) -> String {
    let tip = format!("surt/{SUBJECT}");
    git(worktree, &["merge", "--quiet", "--no-edit", &tip]);
    git(worktree, &["rm", "--quiet", "login.txt", "logout.txt"]);
    commit_file(worktree, "auth.txt", "login\nlogout\n")
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_merge_that_fails_the_checks_sends_the_task_back_and_never_becomes_the_tip() {
    let alone = check(
        "alone",
        &["sh", "-c", "test ! -f login.txt || test ! -f logout.txt"],
    );
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        with_checks(vec![alone]),
    )
    .await;
    let team = rivals([login, logout], &[unify]);

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let Conflicted { key, merged_first } = conflicted(&feature).await;
    let first = feature.task_tip(merged_first).await;
    let first_merge = feature.merge_of(&first).expect("the first merge").sha;
    let conflict = input(
        &team.request(&format!("{key}/build"), 2),
        "conflict",
        "conflict.json",
    )
    .expect("the builder reads it");
    assert_valid(CONFLICT_SCHEMA, &conflict);
    let resolved = feature.task_tip(key).await;
    let rejected = feature.git(&["rev-parse", &format!("{resolved}~2")]);
    let failed_merge = conflict["merge_commit"]
        .as_str()
        .expect("the merge that failed");
    assert_eq!(conflict["kind"], "checks");
    assert_eq!(conflict["integration_tip"], json!(first_merge));
    assert_eq!(conflict["reviewed_commit"], json!(rejected));
    assert_eq!(conflict["checks"][0]["name"], "alone");
    assert_eq!(conflict["checks"][0]["status"], "failed");
    assert_eq!(
        feature.git(&["rev-list", "--parents", "-n", "1", failed_merge]),
        format!("{failed_merge} {first_merge} {rejected}")
    );
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert!(!git_succeeds(
        &feature.root(),
        &["merge-base", "--is-ancestor", failed_merge, &tip]
    ));
    let merge = feature
        .merge_of(&resolved)
        .expect("the resolution is merged");
    assert_eq!((merge.sha, merge.first_parent), (tip, first_merge));
}

async fn conflicts_shown(feature: &Feature) -> Vec<(ItemState, Option<Conflict>)> {
    let statuses = feature
        .project
        .status(StatusScope::Feature(SUBJECT))
        .await
        .expect("the status");
    statuses[0]
        .items
        .iter()
        .map(|task| (task.state, task.conflict.clone()))
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn status_shows_why_a_task_sent_back_by_a_merge_conflict_did_not_merge() {
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        Config::default(),
    )
    .await;

    let driven = feature
        .drive_with(&rivals([shared_login, shared_logout], &[]))
        .await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));
    let mut shown = conflicts_shown(&feature).await;
    shown.sort_by_key(|(state, _)| *state == ItemState::Blocked);
    assert_eq!(
        shown,
        [
            (ItemState::Succeeded, None),
            (
                ItemState::Blocked,
                Some(Conflict::Merge {
                    paths: vec!["shared.txt".to_owned()]
                })
            ),
        ]
    );
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn status_shows_which_checks_failed_on_the_merge_of_a_task_sent_back() {
    let alone = check(
        "alone",
        &["sh", "-c", "test ! -f login.txt || test ! -f logout.txt"],
    );
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        with_checks(vec![alone]),
    )
    .await;

    feature.drive_with(&rivals([login, logout], &[])).await;

    let shown = conflicts_shown(&feature).await;
    assert!(
        shown.contains(&(
            ItemState::Blocked,
            Some(Conflict::Checks {
                failed: vec!["alone".to_owned()]
            })
        )),
        "{shown:?}"
    );
}
