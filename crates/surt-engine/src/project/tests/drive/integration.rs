use std::path::Path;

use super::super::fixtures::SUBJECT;
use super::checks::{check, with_checks};
use super::fixtures::Feature;
use super::repository::commit_file;
use super::team::{INTEGRATED_TASK, Team, builds, built, failures, reviews};
use crate::project::{Config, DriveOutcome};

pub(super) fn integration_branch() -> String {
    format!("refs/heads/surt/{SUBJECT}")
}

pub(super) fn task_branch_of(key: &str) -> String {
    format!("refs/heads/surt/task/{SUBJECT}/{key}")
}

pub(super) fn login(worktree: &Path) -> String {
    commit_file(worktree, "login.txt", "login\n")
}

pub(super) fn logout(worktree: &Path) -> String {
    commit_file(worktree, "logout.txt", "logout\n")
}

pub(super) struct Merge {
    pub(super) sha: String,
    pub(super) first_parent: String,
}

impl Feature {
    pub(super) fn merge_of(&self, reviewed: &str) -> Option<Merge> {
        let listed = self.git(&["rev-list", "--merges", "--parents", &integration_branch()]);
        listed.lines().find_map(|line| {
            let shas: Vec<&str> = line.split(' ').collect();
            match shas.as_slice() {
                [sha, first, second] if *second == reviewed => Some(Merge {
                    sha: (*sha).to_owned(),
                    first_parent: (*first).to_owned(),
                }),
                _ => None,
            }
        })
    }

    pub(super) async fn task_tip(&self, key: &str) -> String {
        let tips: Vec<(String,)> = self
            .dir
            .rows(&format!(
                "SELECT b.tip_sha FROM run_branch b JOIN workflow_run w ON w.id = b.workflow_run_id \
                 WHERE w.item_key = '{key}'"
            ))
            .await;
        tips.into_iter()
            .next()
            .map(|(tip,)| tip)
            .expect("the task branch was recorded")
    }

    pub(super) async fn integrations(&self) -> Vec<(String, u32, String)> {
        self.dir
            .rows(
                "SELECT w.item_key, s.iteration, coalesce(s.outcome, s.state) FROM step_run s \
                 JOIN workflow_run w ON w.id = s.workflow_run_id \
                 WHERE s.step_key = 'integrate' ORDER BY w.item_key, s.iteration",
            )
            .await
    }

    pub(super) async fn recorded_tips(&self) -> Vec<(Option<String>, String, Option<String>)> {
        self.dir
            .rows(
                "SELECT w.item_key, b.tip_sha, b.merged_as_sha FROM run_branch b \
                 JOIN workflow_run w ON w.id = b.workflow_run_id ORDER BY w.item_key",
            )
            .await
    }

    async fn integrate_started_at(&self, key: &str) -> i64 {
        let started: Vec<(String, i64)> = self
            .dir
            .rows(
                "SELECT w.item_key, s.started_at FROM step_run s \
                 JOIN workflow_run w ON w.id = s.workflow_run_id \
                 WHERE s.step_key = 'integrate' AND s.outcome = 'merged'",
            )
            .await;
        started
            .into_iter()
            .find(|(item, _)| item == key)
            .map(|(_, at)| at)
            .expect("the task was merged")
    }
}

fn independent_tasks() -> Team {
    Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on("B/build", 1, 1, builds(logout))
        .on("B/review", 1, 1, reviews("approved"))
}

#[tokio::test(flavor = "multi_thread")]
async fn every_approved_task_enters_the_integration_branch_as_a_merge_of_its_reviewed_sha() {
    let feature = built(
        INTEGRATED_TASK,
        &[("A", &[]), ("B", &[])],
        Config::default(),
    )
    .await;

    let driven = feature.drive_with(&independent_tasks()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(
        feature.integrations().await,
        [
            ("A".to_owned(), 1, "merged".to_owned()),
            ("B".to_owned(), 1, "merged".to_owned())
        ]
    );
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    let mut merges = Vec::new();
    for key in ["A", "B"] {
        let reviewed = feature.task_tip(key).await;
        let merge = feature
            .merge_of(&reviewed)
            .expect("a merge of the reviewed sha");
        assert_eq!(
            feature.git(&["log", "-1", "--format=%B", &merge.sha]),
            format!(
                "Merge task {key} into surt/{SUBJECT}\n\nSurt-Run: {SUBJECT}\nSurt-Task: {key}\nSurt-Reviewed-Commit: {reviewed}"
            )
        );
        let started = feature.integrate_started_at(key).await;
        assert_eq!(
            feature.git(&["log", "-1", "--format=%at %ct", &merge.sha]),
            format!("{started} {started}")
        );
        merges.push(merge);
    }
    let first = merges
        .iter()
        .find(|merge| merge.first_parent == feature.base())
        .expect("one task merged onto the base");
    let second = merges
        .iter()
        .find(|merge| merge.first_parent == first.sha)
        .expect("the other merged onto the first merge");
    assert_eq!(second.sha, tip);
    let tips = feature.recorded_tips().await;
    assert_eq!(tips[0], (None, tip, None));
    assert_eq!(tips.len(), 3);
    for (_, _, merged_as) in &tips[1..] {
        assert!(
            merges
                .iter()
                .any(|merge| Some(&merge.sha) == merged_as.as_ref()),
            "{tips:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn integration_reruns_the_checks_on_the_merge_only_when_the_tip_moved_since_the_task_started()
{
    let config = with_checks(vec![check("head", &["git", "rev-parse", "HEAD"])]);
    let feature = built(INTEGRATED_TASK, &[("A", &[]), ("B", &[])], config).await;

    let driven = feature.drive_with(&independent_tasks()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let logs: Vec<(String, String)> = feature
        .dir
        .rows(
            "SELECT w.item_key, e.id FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE s.step_key = 'integrate' ORDER BY e.finished_at, e.id",
        )
        .await;
    let rechecked: Vec<Option<String>> = logs
        .iter()
        .map(|(_, execution)| feature.integration_log(execution))
        .collect();
    let tip = feature
        .sha(&integration_branch())
        .expect("the integration branch");
    assert_eq!(rechecked[0], None);
    let log = rechecked[1]
        .as_deref()
        .expect("the second integration re-checked");
    assert!(log.contains(&format!("{tip}\n")), "{log}");
    assert!(failures(&feature, "integrate").await.is_empty());
}

impl Feature {
    pub(super) fn integration_log(&self, execution: &str) -> Option<String> {
        let path = self
            .root()
            .join(".surt/logs")
            .join(SUBJECT)
            .join(execution)
            .join("output.log");
        std::fs::read_to_string(path).ok()
    }
}
