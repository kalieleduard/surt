use std::num::NonZeroU32;

use tokio::sync::{Notify, mpsc};

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, draft, label, summary};
use super::integration::login;
use super::recovery::Started;
use super::team::{INTEGRATED_TASK, Team, builds, built, reviews};
use crate::project::{CancelImpact, Cancellation, Config, DriveOutcome, Error, Progress};
use crate::runtime::{Behavior, FakeRuntime, Script, StopSignal};

pub(super) struct Cancelled {
    pub(super) outcome: Result<Cancellation, Error>,
    pub(super) progress: Vec<Progress>,
}

impl Feature {
    pub(super) async fn cancel(&self) -> Cancelled {
        let (sink, mut events) = mpsc::unbounded_channel();
        let outcome = self.project.cancel(&self.run, sink).await;
        let mut progress = Vec::new();
        while let Ok(event) = events.try_recv() {
            progress.push(event);
        }
        Cancelled { outcome, progress }
    }

    async fn states(&self, table: &str) -> Vec<String> {
        let rows: Vec<(String,)> = self
            .dir
            .rows(&format!("SELECT state FROM {table} ORDER BY rowid"))
            .await;
        rows.into_iter().map(|(state,)| state).collect()
    }

    async fn root_cleaned_at(&self) -> Option<i64> {
        let rows: Vec<(Option<i64>,)> = self
            .dir
            .rows("SELECT cleaned_at FROM workflow_run WHERE parent_step_run_id IS NULL")
            .await;
        rows[0].0
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_a_driven_run_asks_its_driver_to_stop_and_returns_once_it_has() {
    let feature = Feature::review().await;
    let runtime = Started {
        fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
        launched: Notify::new(),
    };
    let (sink, _events) = mpsc::unbounded_channel();
    let cancelling = async {
        runtime.launched.notified().await;
        feature.cancel().await
    };

    let (driven, cancelled) = tokio::join!(
        feature
            .project
            .drive(&feature.run, &runtime, sink, StopSignal::new()),
        cancelling
    );

    assert_eq!(driven.expect("drive returns"), DriveOutcome::Cancelled);
    assert_eq!(
        cancelled.outcome.expect("cancel returns"),
        Cancellation::Cancelled
    );
    assert_eq!(
        cancelled.progress,
        [Progress::CancelRequested {
            driver: Some(std::process::id())
        }]
    );
    assert_eq!(feature.states("execution").await, ["cancelled"]);
    assert_eq!(feature.states("workflow_run").await, ["cancelled"]);
    let requests: Vec<(String,)> = feature
        .dir
        .rows(
            "SELECT w.cancel_requested_by FROM workflow_run w JOIN event e ON e.workflow_run_id = w.id \
             WHERE e.kind = 'run.cancel_requested'",
        )
        .await;
    let actor = feature.project.actor().await.expect("the actor");
    assert_eq!(requests, [(actor,)]);
    assert!(feature.root_cleaned_at().await.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_impact_of_a_cancel_counts_live_executions_and_the_tasks_merged_that_stay() {
    let config = Config {
        max_attempts: NonZeroU32::MIN,
        ..Config::default()
    };
    let feature = built(INTEGRATED_TASK, &[("A", &[]), ("B", &[])], config).await;
    let team = Team::new()
        .on("A/build", 1, 1, builds(login))
        .on("A/review", 1, 1, reviews("approved"))
        .on(
            "B/build",
            1,
            1,
            Box::new(|_| Behavior::Crash { exit_code: 1 }),
        );
    let blocked = feature.drive_with(&team).await;
    assert!(matches!(
        blocked.outcome.expect("drive returns"),
        DriveOutcome::Blocked { .. }
    ));

    let impact = feature
        .project
        .cancel_impact(&feature.run)
        .await
        .expect("the impact");

    assert_eq!(
        impact,
        CancelImpact {
            executions: 0,
            items: 2,
            merged: 1,
            integration_branch: Some(format!("surt/{SUBJECT}")),
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_impact_of_a_cancel_while_driving_counts_the_running_execution() {
    let feature = Feature::review().await;
    feature.dropped_mid_execution().await;

    let impact = feature
        .project
        .cancel_impact(&feature.run)
        .await
        .expect("the impact");

    assert_eq!(
        impact,
        CancelImpact {
            executions: 1,
            items: 0,
            merged: 0,
            integration_branch: None,
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_an_already_cancelled_run_only_finishes_its_pending_cleanup() {
    let feature = Feature::review().await;
    feature.dropped_mid_execution().await;
    feature.cancel().await.outcome.expect("the first cancel");
    feature
        .dir
        .execute("UPDATE workflow_run SET cleaned_at = NULL")
        .await;

    let again = feature.cancel().await;

    assert_eq!(
        again.outcome.expect("cancel returns"),
        Cancellation::AlreadyCancelled
    );
    assert!(
        matches!(again.progress[..], [Progress::CleanedUp(_)]),
        "{:?}",
        again.progress
    );
    assert!(feature.root_cleaned_at().await.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_a_succeeded_run_changes_nothing() {
    let feature = Feature::review().await;
    feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft 1\n")))
        .await
        .outcome
        .expect("the draft reaches its gate");
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the draft is approved");
    let published = feature
        .drive(Script::new().on("publish", 1, 1, summary("published\n")))
        .await;
    assert_eq!(
        published.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );

    let cancelled = feature.cancel().await;

    assert_eq!(
        cancelled.outcome.expect("cancel returns"),
        Cancellation::AlreadySucceeded
    );
    assert!(cancelled.progress.is_empty(), "{:?}", cancelled.progress);
    assert_eq!(feature.states("workflow_run").await, ["succeeded"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_a_crashed_run_settles_what_its_driver_left_then_cancels_and_cleans_up() {
    let feature = Feature::review().await;
    feature.dropped_mid_execution().await;

    let cancelled = feature.cancel().await;

    assert_eq!(
        cancelled.outcome.expect("cancel returns"),
        Cancellation::Cancelled
    );
    assert_eq!(
        cancelled.progress[0],
        Progress::InterruptedWithDriver {
            step: label("write", Some(1)),
            attempt: 1
        }
    );
    assert!(
        matches!(cancelled.progress[1..], [Progress::CleanedUp(_)]),
        "{:?}",
        cancelled.progress
    );
    assert_eq!(feature.states("execution").await, ["interrupted"]);
    assert_eq!(feature.states("workflow_run").await, ["cancelled"]);
    assert!(
        feature
            .states("step_run")
            .await
            .iter()
            .all(|state| state == "cancelled"),
    );
    assert!(feature.root_cleaned_at().await.is_some());
}
