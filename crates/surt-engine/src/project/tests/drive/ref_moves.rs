use std::path::Path;

use super::fixtures::{Feature, label};
use super::integration::{integration_branch, login, task_branch_of};
use super::repository::git;
use super::team::{INTEGRATED_TASK, Team, builds, built, failures, reviews};
use crate::project::{Config, DriveOutcome, StepLabel};

fn moving_the_integration_branch(worktree: &Path) -> String {
    let sha = login(worktree);
    git(worktree, &["update-ref", &integration_branch(), &sha]);
    sha
}

fn integrate_of_a() -> StepLabel {
    StepLabel {
        item: Some("A".to_owned()),
        ..label("integrate", Some(1))
    }
}

async fn blocked_by_a_moved_branch() -> Feature {
    let feature = built(INTEGRATED_TASK, &[("A", &[])], Config::default()).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(moving_the_integration_branch))
        .on("A/review", 1, 1, reviews("approved"));

    let driven = feature.drive_with(&team).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked {
            steps: vec![integrate_of_a()]
        }
    );
    feature
}

impl Feature {
    async fn intend_move(&self, old: &str, new: &str) {
        let sql = format!(
            "INSERT INTO ref_move (step_run_id, ref_name, old_sha, new_sha, created_at) \
             SELECT id, '{}', '{old}', '{new}', 0 FROM step_run WHERE step_key = 'integrate'",
            integration_branch()
        );
        self.dir.execute(&sql).await;
    }

    async fn pending_moves(&self) -> Vec<(String,)> {
        self.dir.rows("SELECT new_sha FROM ref_move").await
    }

    fn reviewed(&self) -> String {
        self.sha(&task_branch_of("A")).expect("the task branch")
    }

    fn hand_made_merge(&self) -> String {
        let reviewed = self.reviewed();
        let tree = format!("{reviewed}^{{tree}}");
        self.git(&[
            "commit-tree",
            "--no-gpg-sign",
            &tree,
            "-p",
            &self.base(),
            "-p",
            &reviewed,
            "-m",
            "a merge recorded before a crash",
        ])
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn integration_refuses_to_move_a_branch_that_moved_outside_surt() {
    let feature = blocked_by_a_moved_branch().await;

    let reviewed = feature.reviewed();
    assert_eq!(feature.sha(&integration_branch()), Some(reviewed.clone()));
    assert_eq!(
        failures(&feature, "integrate").await,
        [(
            1,
            1,
            "integration_branch_moved".to_owned(),
            format!(
                "surt/20261017-add-login moved outside surt (expected {}, found {reviewed})",
                feature.base()
            )
        )]
    );
    let tips = feature.recorded_tips().await;
    assert_eq!(tips[0], (None, feature.base(), None));
    assert_eq!(tips[1].2, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_intended_move_found_done_is_confirmed_without_merging_again() {
    let feature = blocked_by_a_moved_branch().await;
    let merge = feature.hand_made_merge();
    feature.intend_move(&feature.base(), &merge).await;
    feature.git(&["update-ref", &integration_branch(), &merge]);

    let driven = feature.drive_with(&Team::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(feature.sha(&integration_branch()), Some(merge.clone()));
    let tips = feature.recorded_tips().await;
    assert_eq!(tips[0], (None, merge.clone(), None));
    assert_eq!(tips[1].2, Some(merge));
    assert!(feature.pending_moves().await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_intended_move_found_undone_is_redone_from_the_start() {
    let feature = blocked_by_a_moved_branch().await;
    let abandoned = feature.hand_made_merge();
    feature.intend_move(&feature.base(), &abandoned).await;
    feature.git(&["update-ref", &integration_branch(), &feature.base()]);

    let driven = feature.drive_with(&Team::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let merge = feature
        .merge_of(&feature.reviewed())
        .expect("the reviewed sha is merged");
    assert_ne!(merge.sha, abandoned);
    assert_eq!(merge.first_parent, feature.base());
    assert_eq!(feature.sha(&integration_branch()), Some(merge.sha.clone()));
    assert_eq!(feature.recorded_tips().await[1].2, Some(merge.sha));
    assert!(feature.pending_moves().await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_intended_move_found_elsewhere_blocks_the_integration() {
    let feature = blocked_by_a_moved_branch().await;
    let merge = feature.hand_made_merge();
    feature.intend_move(&feature.base(), &merge).await;

    let driven = feature.drive_with(&Team::new()).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Blocked {
            steps: vec![integrate_of_a()]
        }
    );
    let refused = failures(&feature, "integrate").await;
    assert_eq!(refused.len(), 2, "{refused:?}");
    assert_eq!(refused[1].2, "integration_branch_moved");
    assert_eq!(feature.sha(&integration_branch()), Some(feature.reviewed()));
    assert!(feature.pending_moves().await.is_empty());
}
