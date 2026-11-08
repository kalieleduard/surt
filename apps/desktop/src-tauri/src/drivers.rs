mod ending;

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use surt_engine::project::{Project, RunId};
use surt_engine::runtime::StopSignal;
use tokio::task::JoinHandle;

pub use ending::Ending;

use crate::workspace::Failure;

#[derive(Clone, Default)]
pub struct Drivers {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
}

struct Entry {
    stop: StopSignal,
    task: Option<JoinHandle<()>>,
    ending: Option<Ending>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverView {
    pub subject: String,
    pub driving: bool,
    pub ending: Option<Ending>,
}

impl Drivers {
    pub fn start(&self, subject: &str, project: Project, run: RunId) -> Result<(), Failure> {
        self.start_with(subject, move |stop| async move {
            ending::drive(&project, &run, stop).await
        })
    }

    pub fn start_with<F, W>(&self, subject: &str, drive: F) -> Result<(), Failure>
    where
        F: FnOnce(StopSignal) -> W,
        W: Future<Output = Ending> + Send + 'static,
    {
        let mut entries = self.lock();
        if entries
            .get(subject)
            .is_some_and(|entry| entry.task.is_some())
        {
            return Err(Failure::AlreadyDriving(subject.to_owned()));
        }
        let stop = StopSignal::new();
        let work = drive(stop.clone());
        let registry = self.clone();
        let name = subject.to_owned();
        let task = tokio::spawn(async move {
            let ending = work.await;
            registry.end(&name, ending);
        });
        let entry = Entry {
            stop,
            task: Some(task),
            ending: None,
        };
        entries.insert(subject.to_owned(), entry);
        Ok(())
    }

    pub fn views(&self) -> Vec<DriverView> {
        let mut views: Vec<DriverView> = self
            .lock()
            .iter()
            .map(|(subject, entry)| DriverView {
                subject: subject.clone(),
                driving: entry.task.is_some(),
                ending: entry.ending.clone(),
            })
            .collect();
        views.sort_by(|left, right| left.subject.cmp(&right.subject));
        views
    }

    pub fn is_driving(&self) -> bool {
        self.lock().values().any(|entry| entry.task.is_some())
    }

    pub async fn stop_all(&self) {
        let tasks: Vec<JoinHandle<()>> = self
            .lock()
            .values_mut()
            .filter_map(|entry| {
                entry.stop.graceful();
                entry.task.take()
            })
            .collect();
        for task in tasks {
            if let Err(error) = task.await {
                eprintln!("a Driver ended abnormally: {error}");
            }
        }
    }

    fn end(&self, subject: &str, ending: Ending) {
        if let Some(entry) = self.lock().get_mut(subject) {
            entry.task = None;
            entry.ending = Some(ending);
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests;
