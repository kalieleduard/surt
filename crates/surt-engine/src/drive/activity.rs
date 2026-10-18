use super::driver::Driver;
use super::graph::ExecutionState;
use super::load::{Loaded, label};
use super::outcome::{ActivityPause, ActivitySnapshot, Progress, RunningStep};
use crate::runtime::AgentRuntime;

/// The agent slots of the project as this Driver sees them.
#[derive(Clone, Copy)]
pub(super) struct Slots {
    pub(super) in_use: usize,
    pub(super) limit: usize,
}

impl<R: AgentRuntime> Driver<'_, R> {
    pub(super) fn observe(
        &self,
        loaded: &Loaded,
        queued: usize,
        slots: Slots,
        paused: Option<ActivityPause>,
    ) {
        let snapshot = ActivitySnapshot {
            running: running(loaded),
            queued,
            slots_in_use: slots.in_use,
            slots_limit: slots.limit,
            paused,
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
