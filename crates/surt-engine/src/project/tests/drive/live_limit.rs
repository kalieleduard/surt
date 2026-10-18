use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::{mpsc, watch};

use super::items::limited;
use super::planned::{TASK_WORKFLOW, approved_with, work};
use crate::project::{ActivitySnapshot, DriveOutcome, Progress};
use crate::runtime::{
    AgentRuntime, FakeRuntime, RuntimeCompletion, RuntimeError, RuntimeEventSink, RuntimeRequest,
    Script, StopSignal,
};

const CEILING: Duration = Duration::from_secs(60);

/// Agents of the `held` steps run until released; the others finish at once.
struct Gate {
    fake: FakeRuntime,
    held: Vec<String>,
    released: watch::Sender<Vec<String>>,
    active: Mutex<usize>,
    started: watch::Sender<Vec<(String, usize)>>,
}

impl Gate {
    fn new(items: &[&str], held: &[&str]) -> Gate {
        let script = items.iter().fold(Script::new(), |script, item| {
            script.on(&format!("{item}/build"), 1, 1, work(item))
        });
        Gate {
            fake: FakeRuntime::new(script),
            held: held.iter().map(|item| format!("{item}/build")).collect(),
            released: watch::Sender::new(Vec::new()),
            active: Mutex::new(0),
            started: watch::Sender::new(Vec::new()),
        }
    }

    fn release(&self, item: &str) {
        self.released
            .send_modify(|released| released.push(format!("{item}/build")));
    }

    /// Each started agent with the number of agents running once it started.
    fn started(&self) -> Vec<(String, usize)> {
        self.started.borrow().clone()
    }

    async fn until_started(&self, count: usize) {
        let mut started = self.started.subscribe();
        tokio::time::timeout(CEILING, started.wait_for(|started| started.len() >= count))
            .await
            .expect("the agents start before the ceiling")
            .map(drop)
            .expect("the started agents are watched");
    }

    fn enter(&self, step: &str) {
        let mut active = self.active.lock().expect("the active count");
        *active += 1;
        let running = *active;
        self.started
            .send_modify(|started| started.push((step.to_owned(), running)));
    }

    fn leave(&self) {
        *self.active.lock().expect("the active count") -= 1;
    }
}

impl AgentRuntime for Gate {
    async fn execute(
        &self,
        request: RuntimeRequest,
        events: RuntimeEventSink,
        stop: StopSignal,
    ) -> Result<RuntimeCompletion, RuntimeError> {
        let step = request.step.step.clone();
        self.enter(&step);
        if self.held.contains(&step) {
            let mut released = self.released.subscribe();
            let _ = released.wait_for(|names| names.contains(&step)).await;
        }
        let completion = self.fake.execute(request, events, stop).await;
        self.leave();
        completion
    }
}

/// The progress of a drive, watched while it runs.
struct Watched {
    events: mpsc::UnboundedReceiver<Progress>,
    seen: Vec<Progress>,
}

impl Watched {
    async fn until(&mut self, wanted: impl Fn(&Progress) -> bool) -> Progress {
        let found = async {
            loop {
                let event = self.events.recv().await.expect("the drive is still going");
                if !matches!(event, Progress::Activity(_)) {
                    self.seen.push(event.clone());
                }
                if wanted(&event) {
                    return event;
                }
            }
        };
        tokio::time::timeout(CEILING, found)
            .await
            .expect("the progress arrives before the ceiling")
    }

    async fn until_activity(&mut self, wanted: impl Fn(&ActivitySnapshot) -> bool) {
        self.until(|event| matches!(event, Progress::Activity(snapshot) if wanted(snapshot)))
            .await;
    }

    async fn rest(mut self) -> Vec<Progress> {
        while let Some(event) = self.events.recv().await {
            if !matches!(event, Progress::Activity(_)) {
                self.seen.push(event);
            }
        }
        self.seen
    }
}

fn running(snapshot: &ActivitySnapshot) -> Vec<&str> {
    snapshot
        .running
        .iter()
        .filter_map(|running| running.step.item.as_deref())
        .collect()
}

/// Saves the project configuration the way an editor does: whole, in one rename.
fn save_config(root: &Path, text: &str) {
    let saved = root.join(".surt").join("config.toml.saving");
    fs::write(&saved, text).expect("the configuration is written");
    fs::rename(&saved, root.join(".surt").join("config.toml")).expect("the configuration is saved");
}

#[tokio::test(flavor = "multi_thread")]
async fn lowering_the_limit_mid_run_lets_the_running_agents_finish_then_launches_one_at_a_time() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[]), ("C", &[]), ("D", &[])];
    let feature = approved_with(TASK_WORKFLOW, items, limited(2)).await;
    let runtime = Gate::new(&["A", "B", "C", "D"], &["A", "B"]);
    let (sink, events) = mpsc::unbounded_channel();
    let mut watched = Watched {
        events,
        seen: Vec::new(),
    };

    let drive = feature
        .project
        .drive(&feature.run, &runtime, sink, StopSignal::new());
    let scenario = async {
        runtime.until_started(2).await;
        save_config(&feature.dir.path(), "max_concurrent_agents = 1\n");
        watched
            .until_activity(|snapshot| running(snapshot).len() == 2 && snapshot.slots_limit == 1)
            .await;
        runtime.release("B");
        watched
            .until_activity(|snapshot| running(snapshot) == ["A"] && snapshot.queued == 2)
            .await;
        runtime.release("A");
        watched.rest().await
    };
    let (outcome, _) = tokio::join!(drive, scenario);

    assert_eq!(outcome.expect("drive returns"), DriveOutcome::Succeeded);
    assert_eq!(
        runtime.started(),
        [
            ("A/build".to_owned(), 1),
            ("B/build".to_owned(), 2),
            ("C/build".to_owned(), 1),
            ("D/build".to_owned(), 1),
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn raising_the_limit_mid_run_launches_a_second_agent_without_restarting() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[])];
    let feature = approved_with(TASK_WORKFLOW, items, limited(1)).await;
    let runtime = Gate::new(&["A", "B"], &["A", "B"]);
    let (sink, events) = mpsc::unbounded_channel();
    let mut watched = Watched {
        events,
        seen: Vec::new(),
    };

    let drive = feature
        .project
        .drive(&feature.run, &runtime, sink, StopSignal::new());
    let scenario = async {
        watched
            .until_activity(|snapshot| {
                running(snapshot) == ["A"] && snapshot.queued == 1 && snapshot.slots_limit == 1
            })
            .await;
        save_config(&feature.dir.path(), "max_concurrent_agents = 2\n");
        runtime.until_started(2).await;
        runtime.release("A");
        runtime.release("B");
        watched.rest().await
    };
    let (outcome, _) = tokio::join!(drive, scenario);

    assert_eq!(outcome.expect("drive returns"), DriveOutcome::Succeeded);
    assert_eq!(
        runtime.started(),
        [("A/build".to_owned(), 1), ("B/build".to_owned(), 2)]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_invalid_configuration_mid_run_keeps_the_limit_and_warns_once_until_it_parses_again() {
    let items: &[(&str, &[&str])] = &[("A", &[]), ("B", &[]), ("C", &[])];
    let feature = approved_with(TASK_WORKFLOW, items, limited(1)).await;
    let runtime = Gate::new(&["A", "B", "C"], &["A", "B", "C"]);
    let (sink, events) = mpsc::unbounded_channel();
    let mut watched = Watched {
        events,
        seen: Vec::new(),
    };
    let invalid = |event: &Progress| matches!(event, Progress::ConfigInvalid { .. });

    let drive = feature
        .project
        .drive(&feature.run, &runtime, sink, StopSignal::new());
    let scenario = async {
        watched
            .until_activity(|snapshot| running(snapshot) == ["A"] && snapshot.queued == 2)
            .await;
        save_config(&feature.dir.path(), "max_concurrent_agents = \n");
        watched.until(invalid).await;
        runtime.release("A");
        watched
            .until_activity(|snapshot| {
                running(snapshot) == ["B"] && snapshot.queued == 1 && snapshot.slots_limit == 1
            })
            .await;
        save_config(&feature.dir.path(), "max_concurrent_agents = 2\n");
        runtime.until_started(3).await;
        save_config(&feature.dir.path(), "max_concurrent_agents = [\n");
        watched.until(invalid).await;
        runtime.release("B");
        runtime.release("C");
        watched.rest().await
    };
    let (outcome, seen) = tokio::join!(drive, scenario);

    assert_eq!(outcome.expect("drive returns"), DriveOutcome::Succeeded);
    let config = feature.dir.path().join(".surt").join("config.toml");
    let warnings: Vec<_> = seen
        .iter()
        .filter_map(|event| match event {
            Progress::ConfigInvalid {
                path,
                line,
                reason,
                limit,
            } => Some((path.clone(), *line, !reason.is_empty(), *limit)),
            _ => None,
        })
        .collect();
    assert_eq!(
        warnings,
        [
            (config.clone(), Some(1), true, 1),
            (config, Some(1), true, 2)
        ]
    );
    assert_eq!(
        runtime.started(),
        [
            ("A/build".to_owned(), 1),
            ("B/build".to_owned(), 1),
            ("C/build".to_owned(), 2),
        ]
    );
}
