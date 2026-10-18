use std::collections::BTreeSet;
use std::sync::Mutex;

use serde_json::json;
use tokio::sync::{mpsc, watch};

use super::super::fixtures::SUBJECT;
use super::evidence::{Committing, change, committed_catalog};
use super::fixtures::{ACTOR, Feature};
use super::planned::{TASK_WORKFLOW, approved, plan, review, work};
use crate::project::{
    ActivityPause, ActivitySnapshot, Config, DriveOutcome, ItemState, LaunchPause, Progress,
    RunSelector, RunState, RunStatus, StatusScope,
};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopSignal,
};

const RESET_DELAY_SECONDS: i64 = 2;
const DISTANT_RESET_SECONDS: i64 = 600;
const CEILING: std::time::Duration = std::time::Duration::from_secs(60);

const CHECKED_TASK: &str = r#"
schema = 1
name = "task"

[steps.setup]
kind = "operation"
run = "git.create_child_worktree"

[steps.build]
kind = "agent"
agent = "builder"
needs = ["setup"]

[steps.check]
kind = "operation"
run = "checks.run"
needs = ["build"]

[steps.review]
kind = "agent"
agent = "reviewer"
needs = ["check"]
"#;

/// Fails the first attempt of `limited` with the Fake's `limit_failure`,
/// its reset time `delay` seconds after the failure, and holds the first
/// attempt of `held` until it is released; everything else goes to `inner`.
struct RateLimited<R> {
    inner: R,
    limited: &'static str,
    delay: i64,
    resets_at: Mutex<Option<i64>>,
    held: Option<&'static str>,
    released: watch::Sender<bool>,
}

impl<R> RateLimited<R> {
    fn new(inner: R, limited: &'static str, delay: i64) -> RateLimited<R> {
        RateLimited {
            inner,
            limited,
            delay,
            resets_at: Mutex::new(None),
            held: None,
            released: watch::Sender::new(false),
        }
    }

    fn holding(self, held: &'static str) -> RateLimited<R> {
        RateLimited {
            held: Some(held),
            ..self
        }
    }

    fn resets_at(&self) -> i64 {
        self.resets_at
            .lock()
            .expect("the reset time")
            .expect("the limit was hit")
    }

    fn release(&self) {
        self.released.send_replace(true);
    }
}

impl<R: AgentRuntime> AgentRuntime for RateLimited<R> {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let step = request.step.step.clone();
        if request.step.attempt == 1 && step == self.limited {
            let resets_at = now() + self.delay;
            *self.resets_at.lock().expect("the reset time") = Some(resets_at);
            let script = Script::new().on(&step, 1, 1, limited(resets_at));
            return FakeRuntime::new(script)
                .execute(request, events, stop)
                .await;
        }
        if request.step.attempt == 1 && Some(step.as_str()) == self.held {
            let mut released = self.released.subscribe();
            let _ = released.wait_for(|released| *released).await;
        }
        self.inner.execute(request, events, stop).await
    }
}
async fn two_checked_tasks() -> Feature {
    let catalog = committed_catalog(CHECKED_TASK);
    let feature = Feature::on_repository(catalog, "committed", Config::default()).await;
    let planned = feature
        .drive(Script::new().on("plan", 1, 1, plan(&[("A", &[]), ("B", &[])])))
        .await
        .outcome
        .expect("the plan reaches its gate");
    assert!(
        matches!(planned, DriveOutcome::AtGate { .. }),
        "{planned:?}"
    );
    feature
        .project
        .approve(&feature.run, None, ACTOR)
        .await
        .expect("the plan is approved");
    feature
}

/// Drives the run, calling `on_pause` when launches pause.
async fn drive_through_pause(
    feature: &Feature,
    runtime: &impl AgentRuntime,
    stop: StopSignal,
    on_pause: impl Fn(),
) -> (DriveOutcome, Vec<Progress>) {
    let (sink, mut events) = mpsc::unbounded_channel();
    let drive = feature.project.drive(&feature.run, runtime, sink, stop);
    let watch = async {
        let mut progress = Vec::new();
        while let Some(event) = events.recv().await {
            if matches!(event, Progress::Paused { .. }) {
                on_pause();
            }
            if !matches!(event, Progress::Activity(_)) {
                progress.push(event);
            }
        }
        progress
    };
    let driven = tokio::time::timeout(CEILING, async { tokio::join!(drive, watch) });
    let (outcome, progress) = driven.await.expect("the drive ends within the ceiling");
    (outcome.expect("drive returns"), progress)
}

async fn agent_starts(feature: &Feature) -> Vec<(String, i64, i64, String)> {
    feature
        .dir
        .rows(
            "SELECT w.item_key || '/' || s.step_key, e.attempt, e.started_at, e.state \
             FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE e.kind = 'agent' AND w.item_key IS NOT NULL ORDER BY e.started_at, e.id",
        )
        .await
}

fn now() -> i64 {
    jiff::Timestamp::now().as_second()
}

async fn status(feature: &Feature) -> RunStatus {
    let mut statuses = feature
        .project
        .status(StatusScope::Feature(SUBJECT, RunSelector::Current))
        .await
        .expect("the status");
    statuses.remove(0)
}

fn limited(resets_at: i64) -> Behavior {
    Behavior::LimitFailure {
        resets_at: Some(resets_at),
    }
}

async fn attempts(feature: &Feature, item: &str) -> Vec<(i64, String, Option<String>, i64)> {
    feature
        .dir
        .rows(&format!(
            "SELECT e.attempt, e.state, e.failure_class, e.started_at FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE s.step_key = 'build' AND w.item_key = '{item}' ORDER BY e.attempt"
        ))
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_transient_failure_pauses_agent_launches_until_the_reset_and_spends_no_attempt() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[])]).await;
    let script = Script::new().on("A/build", 1, 2, work("a"));
    let runtime = RateLimited::new(FakeRuntime::new(script), "A/build", RESET_DELAY_SECONDS);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let resets_at = runtime.resets_at();
    let attempts = attempts(&feature, "A").await;
    let shape: Vec<_> = attempts
        .iter()
        .map(|(attempt, state, class, _)| (*attempt, state.as_str(), class.as_deref()))
        .collect();
    assert_eq!(
        shape,
        [(1, "failed", Some("transient")), (2, "succeeded", None)]
    );
    assert!(attempts[1].3 >= resets_at, "{attempts:?} {resets_at}");
    assert_eq!(feature.dir.event_data("step.blocked").await.len(), 0);
    assert_eq!(
        feature.dir.event_data("agents.paused").await,
        [json!({ "until": resets_at, "code": "rate_limit", "waiting": 1 })]
    );
    let resumed = feature.dir.event_data("agents.resumed").await;
    assert_eq!(resumed.len(), 1, "{resumed:?}");
    assert!(resumed[0]["paused_seconds"].is_u64(), "{resumed:?}");
    let pause: Vec<_> = driven
        .progress
        .iter()
        .filter(|event| {
            matches!(
                event,
                Progress::Paused { .. }
                    | Progress::Resumed { .. }
                    | Progress::StepStarted { attempt: 2, .. }
            )
        })
        .collect();
    assert!(
        matches!(
            pause.as_slice(),
            [
                Progress::Paused {
                    until,
                    waiting: 1,
                    backoff: None,
                },
                Progress::Resumed { .. },
                Progress::StepStarted { attempt: 2, .. },
            ] if *until == resets_at
        ),
        "{pause:?}"
    );
}

struct StoppedOncePaused {
    outcome: DriveOutcome,
    status: Option<RunStatus>,
    snapshots: Vec<ActivitySnapshot>,
}

/// Drives the run, reading its status once launches pause, then stops it.
async fn stopped_once_paused(
    feature: &Feature,
    runtime: &impl AgentRuntime,
    stop: StopSignal,
) -> StoppedOncePaused {
    let (sink, mut events) = mpsc::unbounded_channel();
    let drive = feature
        .project
        .drive(&feature.run, runtime, sink, stop.clone());
    let watch = async {
        let (mut status, mut snapshots) = (None, Vec::new());
        while let Some(event) = events.recv().await {
            match event {
                Progress::Paused { .. } if status.is_none() => {
                    status = Some(self::status(feature).await);
                    stop.graceful();
                }
                Progress::Activity(snapshot) => snapshots.push(snapshot),
                _ => {}
            }
        }
        (status, snapshots)
    };
    let driven = tokio::time::timeout(CEILING, async { tokio::join!(drive, watch) });
    let (outcome, (status, snapshots)) = driven.await.expect("the drive ends within the ceiling");
    StoppedOncePaused {
        outcome: outcome.expect("drive returns"),
        status,
        snapshots,
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn status_shows_the_pause_while_the_driver_waits_and_not_once_an_agent_started_after_it() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[])]).await;
    let stop = StopSignal::new();
    let runtime = RateLimited::new(
        FakeRuntime::new(Script::new()),
        "A/build",
        DISTANT_RESET_SECONDS,
    );

    let paused = stopped_once_paused(&feature, &runtime, stop).await;

    assert_eq!(paused.outcome, DriveOutcome::Interrupted);
    let shown = paused.status.expect("launches paused");
    assert_eq!(shown.states, [RunState::Running]);
    assert_eq!(
        shown.paused,
        Some(LaunchPause {
            code: "rate_limit".to_owned(),
            resets_at: Some(runtime.resets_at()),
        })
    );
    let tasks: Vec<_> = shown
        .items
        .iter()
        .map(|task| (task.key.as_str(), task.state, task.step.as_deref()))
        .collect();
    assert_eq!(tasks, [("A", ItemState::Queued, Some("build"))]);

    let resumed = feature
        .drive(Script::new().on("A/build", 1, 2, work("a")))
        .await;
    assert_eq!(
        resumed.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    assert_eq!(status(&feature).await.paused, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_activity_snapshot_carries_the_end_of_the_pause() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[])]).await;
    let runtime = RateLimited::new(
        FakeRuntime::new(Script::new()),
        "A/build",
        DISTANT_RESET_SECONDS,
    );

    let paused = stopped_once_paused(&feature, &runtime, StopSignal::new()).await;

    let pauses: Vec<_> = paused
        .snapshots
        .iter()
        .filter_map(|snapshot| snapshot.paused)
        .collect();
    assert_eq!(
        pauses.first(),
        Some(&ActivityPause {
            until: runtime.resets_at(),
            backoff: None,
        }),
        "{:?}",
        paused.snapshots
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn during_the_pause_no_agent_starts_while_operations_run_and_running_agents_finish() {
    let feature = two_checked_tasks().await;
    let script = Script::new().on("A/review", 1, 1, review("approved")).on(
        "B/review",
        1,
        1,
        review("approved"),
    );
    let committing = Committing::new(script).on(1, 1, change).on(1, 2, change);
    let runtime =
        RateLimited::new(committing, "A/build", 2 * RESET_DELAY_SECONDS).holding("B/build");

    let (outcome, _) =
        drive_through_pause(&feature, &runtime, StopSignal::new(), || runtime.release()).await;

    assert_eq!(outcome, DriveOutcome::Succeeded);
    let resets_at = runtime.resets_at();
    let starts = agent_starts(&feature).await;
    let shape: Vec<_> = starts
        .iter()
        .map(|(step, attempt, _, state)| (step.as_str(), *attempt, state.as_str()))
        .collect();
    let first_two: BTreeSet<_> = shape[..2].iter().map(|(step, ..)| *step).collect();
    assert_eq!(first_two, ["A/build", "B/build"].into(), "{starts:?}");
    assert!(
        shape.contains(&("A/build", 1, "failed")) && shape.contains(&("B/build", 1, "succeeded")),
        "the build already running when the pause began is accepted: {starts:?}"
    );
    for (step, attempt, started_at, _) in &starts[2..] {
        assert!(
            *started_at >= resets_at,
            "{step} attempt {attempt} started during the pause: {starts:?}"
        );
    }
    let checks: Vec<(i64,)> = feature
        .dir
        .rows(
            "SELECT e.started_at FROM execution e JOIN step_run s ON s.id = e.step_run_id \
             JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE s.step_key = 'check' AND w.item_key = 'B'",
        )
        .await;
    assert!(
        checks
            .first()
            .is_some_and(|(started_at,)| *started_at < resets_at),
        "task B's check ran during the pause: {checks:?} {resets_at}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stop_during_the_pause_interrupts_and_driving_again_launches_at_once() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[])]).await;
    let stop = StopSignal::new();
    let runtime = RateLimited::new(
        FakeRuntime::new(Script::new()),
        "A/build",
        DISTANT_RESET_SECONDS,
    );

    let (outcome, _) =
        drive_through_pause(&feature, &runtime, stop.clone(), || stop.graceful()).await;

    assert_eq!(outcome, DriveOutcome::Interrupted);
    let again = feature
        .drive(Script::new().on("A/build", 1, 2, work("a")))
        .await;
    assert_eq!(
        again.outcome.expect("drive returns"),
        DriveOutcome::Succeeded
    );
    let attempts = attempts(&feature, "A").await;
    assert_eq!(attempts.len(), 2, "{attempts:?}");
    assert!(attempts[1].3 < runtime.resets_at(), "{attempts:?}");
    assert_eq!(feature.dir.event_data("agents.paused").await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_later_reset_from_an_agent_already_running_moves_the_end_of_the_pause() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &[])]).await;
    let script = Script::new()
        .on(
            "B/build",
            1,
            1,
            Behavior::LimitFailureIn {
                seconds: 2 * RESET_DELAY_SECONDS,
            },
        )
        .on("A/build", 1, 2, work("a"))
        .on("B/build", 1, 2, work("b"));
    let runtime = RateLimited::new(FakeRuntime::new(script), "A/build", RESET_DELAY_SECONDS)
        .holding("B/build");

    let (outcome, _) =
        drive_through_pause(&feature, &runtime, StopSignal::new(), || runtime.release()).await;

    assert_eq!(outcome, DriveOutcome::Succeeded);
    let paused = feature.dir.event_data("agents.paused").await;
    assert_eq!(paused.len(), 2, "{paused:?}");
    assert_eq!(paused[0]["until"], json!(runtime.resets_at()));
    assert!(
        paused[1]["until"].as_i64() > paused[0]["until"].as_i64(),
        "{paused:?}"
    );
    assert_eq!(
        (&paused[0]["waiting"], &paused[1]["waiting"]),
        (&json!(1), &json!(2))
    );
    assert_eq!(feature.dir.event_data("agents.resumed").await.len(), 1);
    for item in ["A", "B"] {
        assert_eq!(attempts(&feature, item).await.len(), 2, "{item}");
    }
}
