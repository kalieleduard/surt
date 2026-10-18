use std::num::NonZeroU32;
use std::sync::Mutex;

use super::super::fixtures::SUBJECT;
use super::fixtures::{ACTOR, Feature, draft, review_catalog};
use crate::project::{
    Config, DriveOutcome, Error, Progress, Project, Refusal, RunState, StopRequest,
};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopSignal,
};

struct Requesting<'a> {
    fake: FakeRuntime,
    project: &'a Project,
    requests: Mutex<Vec<StopRequest>>,
}

impl<'a> Requesting<'a> {
    fn new(script: Script, project: &'a Project) -> Self {
        Requesting {
            fake: FakeRuntime::new(script),
            project,
            requests: Mutex::new(Vec::new()),
        }
    }

    fn requests(&self) -> Vec<StopRequest> {
        self.requests.lock().expect("the requests").clone()
    }
}

impl AgentRuntime for Requesting<'_> {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let requested = self
            .project
            .request_stop(SUBJECT)
            .await
            .expect("a live driver accepts the stop request");
        self.requests.lock().expect("the requests").push(requested);
        self.fake.execute(request, events, stop).await
    }
}

fn waiting_write(attempt: u32) -> Script {
    Script::new().on("write", 1, attempt, Behavior::WaitUntilStopped)
}

async fn stop_requested(feature: &Feature) -> Vec<(Option<i64>, Option<String>)> {
    feature
        .dir
        .rows(
            "SELECT stop_requested_at, stop_requested_by FROM workflow_run \
             WHERE parent_step_run_id IS NULL",
        )
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stop_request_makes_the_live_driver_interrupt_its_executions() {
    let feature = Feature::review().await;
    let runtime = Requesting::new(waiting_write(1), &feature.project);

    let driven = feature.drive_with(&runtime).await;

    assert_eq!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::Interrupted
    );
    let requests = runtime.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].run, feature.run);
    assert_eq!(requests[0].driver, std::process::id());
    assert_eq!(requests[0].executions, 1);
    assert!(driven.progress.contains(&Progress::StopRequested));
    let states: Vec<(String,)> = feature.dir.rows("SELECT state FROM execution").await;
    assert_eq!(states, [("interrupted".to_owned(),)]);
    let actor = feature.project.actor().await.expect("the actor");
    assert_eq!(
        feature.dir.event_data("run.stop_requested").await,
        [serde_json::json!({ "actor": actor })]
    );
    let requested = stop_requested(&feature).await;
    assert!(requested[0].0.is_some());
    assert_eq!(requested[0].1.as_deref(), Some(actor.as_str()));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stop_request_without_a_live_driver_is_refused_with_the_run_state() {
    let feature = Feature::review().await;

    let refused = feature.project.request_stop(SUBJECT).await;

    match refused {
        Err(Error::Refused(Refusal::NoLiveDriver { subject, states })) => {
            assert_eq!(subject, SUBJECT);
            assert_eq!(states, [RunState::Interrupted]);
        }
        other => panic!("expected NoLiveDriver, got {other:?}"),
    }
    assert_eq!(stop_requested(&feature).await, [(None, None)]);
    assert!(
        feature
            .dir
            .event_data("run.stop_requested")
            .await
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_leftover_stop_request_is_cleared_when_the_next_driver_takes_the_lock() {
    let feature = Feature::review().await;
    feature
        .dir
        .execute(&format!(
            "UPDATE workflow_run SET stop_requested_at = 1, stop_requested_by = '{ACTOR}' \
             WHERE parent_step_run_id IS NULL"
        ))
        .await;

    let driven = feature
        .drive(Script::new().on("write", 1, 1, draft("# Draft\n")))
        .await;

    assert!(matches!(
        driven.outcome.expect("drive returns"),
        DriveOutcome::AtGate { .. }
    ));
    assert!(!driven.progress.contains(&Progress::StopRequested));
    assert_eq!(stop_requested(&feature).await, [(None, None)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn stopped_executions_never_spend_an_attempt() {
    let config = Config {
        max_attempts: NonZeroU32::MIN,
        ..Config::default()
    };
    let feature = Feature::with_config(review_catalog(), "review", config).await;
    for attempt in 1..=2 {
        let runtime = Requesting::new(waiting_write(attempt), &feature.project);
        let driven = feature.drive_with(&runtime).await;
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
