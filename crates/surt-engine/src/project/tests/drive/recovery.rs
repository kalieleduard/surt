use tokio::sync::{Notify, mpsc};

use super::fixtures::{Feature, draft};
use crate::project::DriveOutcome;
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopSignal,
};

struct Started {
    fake: FakeRuntime,
    launched: Notify,
}

impl AgentRuntime for Started {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        self.launched.notify_one();
        self.fake.execute(request, events, stop).await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_driver_dropped_mid_execution_frees_the_run_and_its_execution_becomes_interrupted() {
    let feature = Feature::review().await;
    let runtime = Started {
        fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
        launched: Notify::new(),
    };
    let (sink, _events) = mpsc::unbounded_channel();
    tokio::select! {
        outcome = feature.project.drive(&feature.run, &runtime, sink, StopSignal::new()) => {
            panic!("the waiting Execution never ends, got {outcome:?}");
        }
        () = runtime.launched.notified() => {}
    }

    let resumed = feature
        .drive(Script::new().on("write", 1, 2, draft("# Draft 1\n")))
        .await;

    assert!(matches!(
        resumed.outcome.expect("the run is free again"),
        DriveOutcome::AtGate { .. }
    ));
    let attempts: Vec<(i64, String)> = feature
        .dir
        .rows("SELECT attempt, state FROM execution ORDER BY attempt")
        .await;
    assert_eq!(
        attempts,
        [(1, "interrupted".to_owned()), (2, "succeeded".to_owned())]
    );
}
