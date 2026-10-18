use std::num::NonZeroU32;

use tokio::sync::mpsc;

use super::super::fixtures::{ProjectDir, SUBJECT};
use super::fixtures::{Driven, Feature, WRITER, agent, draft, review_catalog};
use super::planned::{TASK_WORKFLOW, approved, work};
use crate::project::{Catalog, Config, DriveOutcome, Progress};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopLevel, StopSignal,
};

enum Interference<'a> {
    Stop(StopLevel),
    Cancel(&'a ProjectDir),
}

struct Interfering<'a> {
    fake: FakeRuntime,
    on_step: &'static str,
    interference: Interference<'a>,
    stop: StopSignal,
}

impl<'a> Interfering<'a> {
    fn new(script: Script, on_step: &'static str, interference: Interference<'a>) -> Self {
        Interfering {
            fake: FakeRuntime::new(script),
            on_step,
            interference,
            stop: StopSignal::new(),
        }
    }

    async fn drive(&self, feature: &Feature) -> Driven {
        let (sink, mut events) = mpsc::unbounded_channel();
        let outcome = feature
            .project
            .drive(&feature.run, self, sink, self.stop.clone())
            .await;
        let mut progress = Vec::new();
        while let Ok(event) = events.try_recv() {
            progress.push(event);
        }
        Driven { outcome, progress }
    }
}

impl AgentRuntime for Interfering<'_> {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        if request.step.step == self.on_step {
            match &self.interference {
                Interference::Stop(StopLevel::Force) => self.stop.force(),
                Interference::Stop(_) => self.stop.graceful(),
                Interference::Cancel(dir) => {
                    dir.execute(
                        "UPDATE workflow_run SET cancel_requested_at = 1, \
                         cancel_requested_by = 'dev@example.com' WHERE parent_step_run_id IS NULL",
                    )
                    .await;
                }
            }
        }
        self.fake.execute(request, events, stop).await
    }
}

fn waiting(steps: &[&str]) -> Script {
    steps.iter().fold(Script::new(), |script, step| {
        script.on(step, 1, 1, Behavior::WaitUntilStopped)
    })
}

async fn executions(feature: &Feature) -> Vec<(String, i64, String)> {
    feature
        .dir
        .rows(
            "SELECT coalesce(w.item_key, s.step_key), e.attempt, e.state FROM execution e \
             JOIN step_run s ON s.id = e.step_run_id JOIN workflow_run w ON w.id = s.workflow_run_id \
             WHERE s.step_key <> 'plan' ORDER BY 1, 2",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_graceful_stop_interrupts_every_running_agent_and_returns_interrupted() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &[])]).await;
    let runtime = Interfering::new(
        waiting(&["A/build", "B/build"]),
        "B/build",
        Interference::Stop(StopLevel::Graceful),
    );

    let driven = runtime.drive(&feature).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Interrupted
    );
    assert_eq!(
        executions(&feature).await,
        [
            ("A".to_owned(), 1, "interrupted".to_owned()),
            ("B".to_owned(), 1, "interrupted".to_owned())
        ]
    );
    let interrupted = driven
        .progress
        .iter()
        .filter(|progress| matches!(progress, Progress::AttemptInterrupted { .. }))
        .count();
    assert_eq!(interrupted, 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn interrupted_executions_never_spend_an_attempt() {
    let config = Config {
        max_attempts: NonZeroU32::MIN,
        ..Config::default()
    };
    let feature = Feature::with_config(review_catalog(), "review", config).await;
    for attempt in 1..=2 {
        let runtime = Interfering::new(
            Script::new().on("write", 1, attempt, Behavior::WaitUntilStopped),
            "write",
            Interference::Stop(StopLevel::Graceful),
        );
        let driven = runtime.drive(&feature).await;
        assert_eq!(
            driven.outcome.expect("drive returns"),
            DriveOutcome::Interrupted
        );
    }

    let resumed = feature
        .drive(Script::new().on("write", 1, 3, draft("# Draft\n")))
        .await;

    assert!(
        matches!(
            resumed.outcome.expect("drive returns"),
            DriveOutcome::AtGate { .. }
        ),
        "the third attempt runs although the limit is one failed attempt"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forced_stop_returns_at_once_and_the_next_drive_recovers_the_execution() {
    let feature = Feature::with_config(review_catalog(), "review", Config::default()).await;
    let runtime = Interfering::new(
        waiting(&["write"]),
        "write",
        Interference::Stop(StopLevel::Force),
    );

    let driven = runtime.drive(&feature).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Interrupted
    );
    assert_eq!(
        executions(&feature).await,
        [("write".to_owned(), 1, "running".to_owned())]
    );
    let resumed = feature
        .drive(Script::new().on("write", 1, 2, draft("# Draft\n")))
        .await;
    assert!(matches!(
        resumed.outcome.expect("drive returns"),
        DriveOutcome::AtGate { .. }
    ));
    assert_eq!(
        executions(&feature).await,
        [
            ("write".to_owned(), 1, "interrupted".to_owned()),
            ("write".to_owned(), 2, "succeeded".to_owned())
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cancel_request_is_read_on_the_next_tick_and_cancels_the_whole_run() {
    let feature = approved(TASK_WORKFLOW, &[("A", &[]), ("B", &[])]).await;
    let runtime = Interfering::new(
        Script::new()
            .on("A/build", 1, 1, Behavior::WaitUntilStopped)
            .on("B/build", 1, 1, work("b")),
        "B/build",
        Interference::Cancel(&feature.dir),
    );

    let driven = runtime.drive(&feature).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Cancelled
    );
    assert_eq!(
        executions(&feature).await,
        [
            ("A".to_owned(), 1, "cancelled".to_owned()),
            ("B".to_owned(), 1, "succeeded".to_owned())
        ]
    );
    let runs: Vec<(Option<String>, String)> = feature
        .dir
        .rows("SELECT item_key, state FROM workflow_run ORDER BY item_key")
        .await;
    assert_eq!(
        runs,
        [
            (None, "cancelled".to_owned()),
            (Some("A".to_owned()), "cancelled".to_owned()),
            (Some("B".to_owned()), "cancelled".to_owned())
        ]
    );
    let open: Vec<(i64,)> = feature
        .dir
        .rows("SELECT count(*) FROM step_run WHERE state IN ('pending', 'running', 'blocked')")
        .await;
    assert_eq!(open, [(0,)]);
}

const PARALLEL_WORKFLOW: &str = r#"
schema = 1
name = "parallel"

[steps.prepare]
kind = "operation"
run = "git.create_run_branch"

[steps.write]
kind = "agent"
agent = "writer"
"#;

#[tokio::test(flavor = "multi_thread")]
async fn a_graceful_stop_lets_a_running_operation_finish() {
    let feature = Feature::on_repository(
        Catalog::new()
            .workflow("parallel", PARALLEL_WORKFLOW)
            .agent("writer", agent(WRITER)),
        "parallel",
        Config::default(),
    )
    .await;
    let runtime = Interfering::new(
        waiting(&["write"]),
        "write",
        Interference::Stop(StopLevel::Graceful),
    );

    let driven = runtime.drive(&feature).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Interrupted
    );
    assert_eq!(
        executions(&feature).await,
        [
            ("prepare".to_owned(), 1, "succeeded".to_owned()),
            ("write".to_owned(), 1, "interrupted".to_owned())
        ]
    );
    assert_eq!(
        feature.sha(&format!("refs/heads/surt/{SUBJECT}")),
        Some(feature.base())
    );
}
