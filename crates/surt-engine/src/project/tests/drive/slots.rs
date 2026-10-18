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
    ActivitySnapshot, DriveOutcome, Error, NewRun, Progress, ProgressSink, Project, RunId,
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

#[tokio::test(flavor = "multi_thread")]
async fn two_drivers_of_one_project_share_its_agent_slots_and_a_dropped_driver_frees_its_slot() {
    let dir = ProjectDir::new();
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
            .expect("the run is created");
        projects.push(project);
        runs.push(run);
        runtimes.push(Counted {
            driver,
            agents: Arc::clone(&agents),
            fake: FakeRuntime::new(Script::new().on("write", 1, 1, Behavior::WaitUntilStopped)),
        });
    }
    let stops = [StopSignal::new(), StopSignal::new()];
    let mut receivers = Vec::new();
    let mut drivers = Vec::new();
    for driver in 0..2 {
        let (sink, events) = mpsc::unbounded_channel();
        receivers.push(events);
        drivers.push(Some(Box::pin(driving(
            &projects[driver],
            &runs[driver],
            &runtimes[driver],
            sink,
            stops[driver].clone(),
        ))));
    }

    let waiter = until(&mut drivers, async {
        let [first, second] = receivers.as_mut_slice() else {
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
    .await;
    let holder = 1 - waiter;
    assert_eq!(agents.started(), [holder]);

    drivers[holder] = None;
    let mut started = agents.started.subscribe();
    until(&mut drivers, started.wait_for(|started| started.len() == 2))
        .await
        .map(drop)
        .expect("the started agents are watched");
    stops[waiter].graceful();
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
