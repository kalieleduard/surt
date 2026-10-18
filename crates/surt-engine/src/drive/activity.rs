use super::driver::Driver;
use super::graph::ExecutionState;
use super::load::{Loaded, label};
use super::outcome::{ActivitySnapshot, Progress, RunningStep};
use crate::runtime::AgentRuntime;

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn observe(&self, loaded: &Loaded, queued: usize, in_use: usize, limit: usize) {
        let snapshot = ActivitySnapshot {
            running: running(loaded),
            queued,
            slots_in_use: in_use,
            slots_limit: limit,
        };
        let Ok(mut last) = self.activity.lock() else {
            return;
        };
        if last.as_ref() != Some(&snapshot) {
            *last = Some(snapshot.clone());
            self.report(Progress::Activity(snapshot));
        }
    }
}

fn running(loaded: &Loaded) -> Vec<RunningStep> {
    loaded
        .rows
        .executions
        .iter()
        .filter(|execution| execution.state == ExecutionState::Running)
        .filter_map(|execution| {
            let row = loaded
                .rows
                .steps
                .iter()
                .find(|row| row.id == execution.step)?;
            Some(RunningStep {
                step: label(row),
                started_at: execution.started_at?,
            })
        })
        .collect()
}
