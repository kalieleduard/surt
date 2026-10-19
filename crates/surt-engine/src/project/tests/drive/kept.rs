use super::super::fixtures::SUBJECT;
use super::cleanup::{
    cleanups, delivered_team, entries, leaves_leftovers_and_crashes,
    leaves_leftovers_and_mismatches,
};
use super::fixtures::{Driven, Feature};
use super::integration::{integration_branch, login, task_branch_of};
use super::team::{INTEGRATED_TASK, builds, built, shipped};
use crate::project::{
    Cancellation, CleanupConfig, CleanupState, Config, Deletion, DriveOutcome, OnSuccess, Progress,
    RunSelector, StatusScope,
};

pub(super) const ITEMS: &[(&str, &[&str])] = &[("A", &[]), ("B", &[])];

fn keeping() -> Config {
    Config {
        cleanup: CleanupConfig {
            on_success: Some(OnSuccess::Keep),
        },
        ..Config::default()
    }
}

fn kept_endings(driven: &Driven) -> usize {
    driven
        .progress
        .iter()
        .filter(|event| **event == Progress::RunKept)
        .count()
}

pub(super) async fn kept() -> Feature {
    let feature = shipped(ITEMS, keeping()).await;
    let team = delivered_team()
        .on("A/build", 1, 1, leaves_leftovers_and_crashes())
        .on("A/build", 1, 2, builds(login));
    let driven = feature.drive_with(&team).await;
    assert_eq!(
        driven.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    assert_eq!(cleanups(&driven), []);
    assert_eq!(kept_endings(&driven), 1);
    feature
}

impl Feature {
    fn assert_everything_kept(&self) {
        for key in ["A", "B"] {
            assert!(self.worktree(key).exists(), "{key}");
            assert!(self.sha(&task_branch_of(key)).is_some(), "{key}");
        }
        assert!(self.surt_refs().contains("refs/surt/salvage/"));
        assert!(self.sha(&integration_branch()).is_some());
        assert!(!entries(&self.working_copy("")).is_empty());
        assert!(!entries(&self.executions_dir()).is_empty());
    }

    async fn cleanup_state(&self) -> Option<CleanupState> {
        let mut statuses = self
            .project
            .status(StatusScope::Subject(SUBJECT, RunSelector::Current))
            .await
            .expect("the status");
        statuses.remove(0).cleanup
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_kept_run_leaves_its_scratch_space_and_refs_and_is_not_cleanup_pending() {
    let feature = kept().await;

    feature.assert_everything_kept();
    assert_eq!(feature.cleaned_at().await, None);
    assert_eq!(feature.events("cleanup.finished").await.len(), 0);
    assert_eq!(feature.events("cleanup.incomplete").await.len(), 0);
    assert_eq!(feature.cleanup_state().await, Some(CleanupState::Kept));
    let subject = feature
        .project
        .resolve_subject(SUBJECT)
        .await
        .expect("the feature");
    assert_eq!((subject.active_run, subject.pending_cleanup), (None, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn driving_a_kept_run_again_removes_nothing() {
    let feature = kept().await;

    let again = feature.drive_with(&delivered_team()).await;

    assert_eq!(
        again.outcome.as_ref().expect("drive returns"),
        &DriveOutcome::Succeeded
    );
    assert_eq!(cleanups(&again), []);
    feature.assert_everything_kept();
    assert_eq!(feature.cleaned_at().await, None);
    assert_eq!(feature.cleanup_state().await, Some(CleanupState::Kept));
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_a_kept_run_removes_it_and_keeps_its_delivered_branch() {
    let feature = kept().await;

    let deleted = feature.project.delete(SUBJECT).await;

    assert_eq!(deleted.expect("the feature is deleted"), Deletion::Deleted);
    for key in ["A", "B"] {
        assert!(!feature.worktree(key).exists(), "{key}");
        assert_eq!(feature.sha(&task_branch_of(key)), None, "{key}");
    }
    assert_eq!(feature.surt_refs(), "");
    assert!(feature.sha(&integration_branch()).is_some());
    assert!(!feature.working_copy("").exists());
    assert_eq!(entries(&feature.executions_dir()), Vec::<String>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_run_ignores_the_keep_policy() {
    let feature = built(INTEGRATED_TASK, ITEMS, keeping()).await;
    let team = delivered_team().on("B/build", 1, 1, leaves_leftovers_and_mismatches());
    let blocked = feature.drive_with(&team).await;
    assert!(
        matches!(blocked.outcome, Ok(DriveOutcome::Blocked { .. })),
        "{:?}",
        blocked.outcome
    );

    let cancelled = feature.cancel().await;

    assert_eq!(
        cancelled.outcome.expect("the run is cancelled"),
        Cancellation::Cancelled
    );
    assert!(
        cancelled
            .progress
            .iter()
            .any(|event| matches!(event, Progress::CleanedUp(_))),
        "{:?}",
        cancelled.progress
    );
    assert!(!cancelled.progress.contains(&Progress::RunKept));
    for key in ["A", "B"] {
        assert!(!feature.worktree(key).exists(), "{key}");
    }
    assert!(feature.cleaned_at().await.is_some());
    assert!(matches!(
        feature.cleanup_state().await,
        Some(CleanupState::Finished { .. })
    ));
}
