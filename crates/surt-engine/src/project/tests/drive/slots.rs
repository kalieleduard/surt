use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use tokio::sync::{mpsc, watch};

use super::super::fixtures::{ProjectDir, new_run};
use super::fixtures::review_catalog;
use super::items::limited;
use crate::project::{
    ActivitySnapshot, AgentSlots, DriveOutcome, Error, NewRun, Progress, ProgressSink, Project,
    RunId, RunSelector, StatusScope,
};
use crate::runtime::{
    AgentRuntime, Behavior, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink,
    RuntimeRequest, Script, StopSignal,
};

const CEILING: Duration = Duration::from_secs(60);
const SUBJECTS: [&str; 2] = ["20261017-add-login", "20261017-fix-cache"];

#[derive(Default)]
struct Agents {
    active: Mutex<usize>,
    peak: Mutex<usize>,
    started: watch::Sender<Vec<usize>>,
}

impl Agents {
    fn enter(&self, driver: usize) -> Running<'_> {
        let mut active = self.active.lock().expect("the active count");
        *active += 1;
        let mut peak = self.peak.lock().expect("the peak");
        *peak = (*peak).max(*active);
        self.started.send_modify(|started| started.push(driver));
        Running { agents: self }
    }

    fn peak(&self) -> usize {
        *self.peak.lock().expect("the peak")
    }

    fn started(&self) -> Vec<usize> {
        self.started.borrow().clone()
    }
}

struct Running<'a> {
    agents: &'a Agents,
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        *self.agents.active.lock().expect("the active count") -= 1;
    }
}

struct Counted {
    driver: usize,
    agents: Arc<Agents>,
    fake: FakeRuntime,
}

impl AgentRuntime for Counted {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let _running = self.agents.enter(self.driver);
        self.fake.execute(request, events, stop).await
    }
}

struct TwoFeatures {
    agents: Arc<Agents>,
    projects: Vec<Project>,
    runs: Vec<RunId>,
    runtimes: Vec<Counted>,
    stops: [StopSignal; 2],
}

impl TwoFeatures {
    async fn new(dir: &ProjectDir) -> TwoFeatures {
        let agents = Arc::new(Agents::default());
        let mut projects = Vec::new();
        let mut runs = Vec::new();
        let mut runtimes = Vec::new();
        for (driver, subject) in SUBJECTS.into_iter().enumerate() {
            let project = dir.project(review_catalog(), limited(1)).await;
            let run = project
                .create_run(NewRun {
                    subject: subject.to_owned(),
                    ..new_run("review")
                })
                .await
                .expect("the run is created")
                .run;
            projects.push(project);
            runs.push(run);
            runtimes.push(Counted {
                driver,
                agents: Arc::clone(&agents),
                fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
            });
        }
        TwoFeatures {
            agents,
            projects,
            runs,
            runtimes,
            stops: [StopSignal::new(), StopSignal::new()],
        }
    }

    fn drive(&self) -> Driven<impl Future<Output = Result<DriveOutcome, Error>> + '_> {
        let mut receivers = Vec::new();
        let mut drivers = Vec::new();
        for driver in 0..2 {
            let (sink, events) = mpsc::unbounded_channel();
            receivers.push(events);
            drivers.push(Some(Box::pin(driving(
                &self.projects[driver],
                &self.runs[driver],
                &self.runtimes[driver],
                sink,
                self.stops[driver].clone(),
            ))));
        }
        Driven { drivers, receivers }
    }
}

struct Driven<F> {
    drivers: Vec<Option<Pin<Box<F>>>>,
    receivers: Vec<mpsc::UnboundedReceiver<Progress>>,
}

fn driving<'a>(
    project: &'a Project,
    run: &'a RunId,
    runtime: &'a Counted,
    progress: ProgressSink,
    stop: StopSignal,
) -> impl Future<Output = Result<DriveOutcome, Error>> + 'a {
    project.drive(run, runtime, progress, stop)
}

async fn until<F, T>(drivers: &mut [Option<Pin<Box<F>>>], condition: impl Future<Output = T>) -> T
where
    F: Future<Output = Result<DriveOutcome, Error>>,
{
    let mut condition = std::pin::pin!(condition);
    let watched = poll_fn(|context| {
        for (index, driver) in drivers.iter_mut().enumerate() {
            if let Some(driver) = driver
                && let Poll::Ready(outcome) = driver.as_mut().poll(context)
            {
                panic!("driver {index} returned early: {outcome:?}");
            }
        }
        condition.as_mut().poll(context)
    });
    tokio::time::timeout(CEILING, watched)
        .await
        .expect("the condition holds before the ceiling")
}

fn waiting_for_a_slot(snapshot: &ActivitySnapshot) -> bool {
    snapshot.running.is_empty()
        && snapshot.queued == 1
        && snapshot.slots_in_use == 1
        && snapshot.slots_limit == 1
}

async fn waiter<F>(
    drivers: &mut [Option<Pin<Box<F>>>],
    receivers: &mut [mpsc::UnboundedReceiver<Progress>],
) -> usize
where
    F: Future<Output = Result<DriveOutcome, Error>>,
{
    until(drivers, async {
        let [first, second] = receivers else {
            unreachable!("two drivers");
        };
        loop {
            let (driver, event) = tokio::select! {
                Some(event) = first.recv() => (0, event),
                Some(event) = second.recv() => (1, event),
            };
            if let Progress::Activity(snapshot) = event
                && waiting_for_a_slot(&snapshot)
            {
                return driver;
            }
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn two_drivers_of_one_project_share_its_agent_slots_and_a_dropped_driver_frees_its_slot() {
    let dir = ProjectDir::new();
    let features = TwoFeatures::new(&dir).await;
    let agents = &features.agents;
    let Driven {
        mut drivers,
        mut receivers,
    } = features.drive();

    let waiter = waiter(&mut drivers, &mut receivers).await;
    let holder = 1 - waiter;
    assert_eq!(agents.started(), [holder]);

    drivers[holder] = None;
    let mut started = agents.started.subscribe();
    until(&mut drivers, started.wait_for(|started| started.len() == 2))
        .await
        .map(drop)
        .expect("the started agents are watched");
    features.stops[waiter].graceful();
    let outcome = drivers[waiter]
        .take()
        .expect("the waiting driver")
        .await
        .expect("the waiting driver returns");

    assert_eq!(outcome, DriveOutcome::Interrupted);
    assert_eq!(agents.started(), [holder, waiter]);
    assert_eq!(agents.peak(), 1);
    let slots: Vec<(Option<i64>,)> = dir.rows("SELECT slot FROM execution ORDER BY id").await;
    assert_eq!(slots, [(Some(0),), (Some(0),)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_status_of_each_of_two_driven_features_counts_the_agent_slots_of_the_whole_project() {
    let dir = ProjectDir::new();
    let features = TwoFeatures::new(&dir).await;
    let Driven {
        mut drivers,
        mut receivers,
    } = features.drive();
    waiter(&mut drivers, &mut receivers).await;

    let reader = &features.projects[0];
    let (each, overview) = until(&mut drivers, async {
        let mut each = Vec::new();
        for subject in SUBJECTS {
            let statuses = reader
                .status(StatusScope::Feature(subject, RunSelector::Current))
                .await
                .expect("the Feature's status reads");
            each.extend(statuses.into_iter().map(|status| status.slots));
        }
        let overview: Vec<AgentSlots> = reader
            .status(StatusScope::Active)
            .await
            .expect("the overview reads")
            .into_iter()
            .map(|status| status.slots)
            .collect();
        (each, overview)
    })
    .await;
    for stop in &features.stops {
        stop.graceful();
    }
    for driver in drivers.into_iter().flatten() {
        let outcome = driver.await.expect("the driver returns");
        assert_eq!(outcome, DriveOutcome::Interrupted);
    }

    let one_of_one = AgentSlots {
        in_use: 1,
        total: 1,
    };
    assert_eq!(each, [one_of_one, one_of_one]);
    assert_eq!(overview, [one_of_one, one_of_one]);
}
