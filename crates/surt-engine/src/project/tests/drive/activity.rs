use std::num::NonZeroU32;

use tokio::sync::{mpsc, watch};

use super::fixtures::{Feature, draft};
use super::planned::{TASK_WORKFLOW, approved_with, work};
use crate::project::{ActivitySnapshot, Config, DriveOutcome, Progress, StepLabel};
use crate::runtime::{
    AgentRuntime, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink, RuntimeRequest,
    Script, StopSignal,
};

struct Held {
    fake: FakeRuntime,
    released: watch::Sender<Vec<String>>,
}

impl Held {
    fn new(script: Script) -> Held {
        Held {
            fake: FakeRuntime::new(script),
            released: watch::Sender::new(Vec::new()),
        }
    }

    fn release(&self, snapshot: &ActivitySnapshot) {
        for running in &snapshot.running {
            let name = match &running.step.item {
                Some(item) => format!("{item}/{}", running.step.key),
                None => running.step.key.clone(),
            };
            self.released.send_modify(|released| released.push(name));
        }
    }
}

impl AgentRuntime for Held {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let mut released = self.released.subscribe();
        let step = request.step.step.clone();
        let _ = released.wait_for(|names| names.contains(&step)).await;
        self.fake.execute(request, events, stop).await
    }
}

struct Observed {
    outcome: DriveOutcome,
    snapshots: Vec<ActivitySnapshot>,
    window: (i64, i64),
}

async fn observe(feature: &Feature, runtime: &Held) -> Observed {
    let before = jiff::Timestamp::now().as_second();
    let (sink, mut events) = mpsc::unbounded_channel();
    let drive = feature
        .project
        .drive(&feature.run, runtime, sink, StopSignal::new());
    let watch = async {
        let mut snapshots = Vec::new();
        while let Some(event) = events.recv().await {
            if let Progress::Activity(snapshot) = event {
                runtime.release(&snapshot);
                snapshots.push(snapshot);
            }
        }
        snapshots
    };
    let (outcome, snapshots) = tokio::join!(drive, watch);
    Observed {
        outcome: outcome.expect("drive returns"),
        snapshots,
        window: (before, jiff::Timestamp::now().as_second()),
    }
}

fn shape(snapshot: &ActivitySnapshot) -> (Vec<StepLabel>, usize, usize, usize) {
    (
        snapshot
            .running
            .iter()
            .map(|running| running.step.clone())
            .collect(),
        snapshot.queued,
        snapshot.slots_in_use,
        snapshot.slots_limit,
    )
}

fn task(item: &str) -> StepLabel {
    StepLabel {
        key: "build".to_owned(),
        item: Some(item.to_owned()),
        iteration: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_step_is_reported_with_its_start_time_and_the_slot_it_holds() {
    let feature = Feature::review().await;
    let runtime = Held::new(Script::new().on("write", 1, 1, draft("# Draft 1\n")));

    let observed = observe(&feature, &runtime).await;

    assert!(matches!(observed.outcome, DriveOutcome::AtGate { .. }));
    let write = StepLabel {
        key: "write".to_owned(),
        item: None,
        iteration: Some(1),
    };
    let shapes: Vec<_> = observed.snapshots.iter().map(shape).collect();
    assert_eq!(shapes, [(vec![write], 0, 1, 3)]);
    let started = observed.snapshots[0].running[0].started_at;
    let (before, after) = observed.window;
    assert!((before..=after).contains(&started), "{started}");
}

#[tokio::test(flavor = "multi_thread")]
async fn items_waiting_for_a_slot_are_counted_as_queued_and_each_change_is_reported_once() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[]), ("C", &[])];
    let config = Config {
        max_concurrent_agents: NonZeroU32::MIN,
        ..Config::default()
    };
    let feature = approved_with(TASK_WORKFLOW, items, config).await;
    let runtime = Held::new(
        Script::new()
            .on("A/build", 1, 1, work("a"))
            .on("B/build", 1, 1, work("b"))
            .on("C/build", 1, 1, work("c")),
    );

    let observed = observe(&feature, &runtime).await;

    assert_eq!(observed.outcome, DriveOutcome::Succeeded);
    let shapes: Vec<_> = observed.snapshots.iter().map(shape).collect();
    assert_eq!(
        shapes,
        [
            (vec![task("A")], 2, 1, 1),
            (vec![task("B")], 1, 1, 1),
            (vec![task("C")], 0, 1, 1),
        ]
    );
}
