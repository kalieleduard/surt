use std::fmt::Write;

use surt_engine::project::{AgentSlots, Conflict, ItemState, ItemStatus};

use super::line;
use crate::driving::{local_time, short};
use crate::states::item_state_name;
use crate::symbols::{BLOCKED, ERROR, PAUSED, SEPARATOR, STARTED, STOPPED, SUCCESS, WAITING};

const LIST_SEPARATOR: &str = ", ";
const WORD_WIDTH: usize = 9;
const TASK_STEP: &str = "implement";

pub(super) fn render(text: &mut String, tasks: &[ItemStatus], slots: AgentSlots, paused: bool) {
    if tasks.is_empty() {
        return;
    }
    line(text, TASK_STEP);
    let width = tasks
        .iter()
        .map(|task| task.key.chars().count())
        .max()
        .unwrap_or_default();
    for task in tasks {
        let (mark, word) = look(task);
        let entry = format!(
            "    {:<width$}  {mark} {word:<WORD_WIDTH$} {}",
            task.key,
            details(task, slots, paused)
        );
        let _ = writeln!(text, "{}", entry.trim_end());
    }
}

fn look(task: &ItemStatus) -> (&'static str, String) {
    let step = || task.step.clone().unwrap_or_default();
    let named = || item_state_name(task.state).to_owned();
    match task.state {
        ItemState::Succeeded => (SUCCESS, "merged".to_owned()),
        ItemState::Running => (STARTED, step()),
        ItemState::Queued => (WAITING, named()),
        ItemState::Pending => (PAUSED, named()),
        ItemState::Interrupted => (STOPPED, "stopped".to_owned()),
        ItemState::Blocked => (BLOCKED, named()),
        ItemState::Cancelled => (ERROR, named()),
    }
}

fn details(task: &ItemStatus, slots: AgentSlots, paused: bool) -> String {
    let mut parts = Vec::new();
    let iteration = task.iteration.map(|iteration| format!("iter {iteration}"));
    match task.state {
        ItemState::Succeeded => {
            parts.extend(iteration);
            parts.extend(
                task.merged_as
                    .as_deref()
                    .map(|sha| format!("as {}", short(sha))),
            );
        }
        ItemState::Running => {
            parts.extend(iteration);
            parts.extend(attempt(task));
            parts.extend(
                task.since
                    .and_then(local_time)
                    .map(|time| format!("since {time}")),
            );
        }
        ItemState::Queued | ItemState::Interrupted | ItemState::Blocked => {
            parts.extend(task.step.as_deref().map(|step| match &iteration {
                Some(iteration) => format!("{step} {iteration}"),
                None => step.to_owned(),
            }));
            parts.extend(attempt(task));
        }
        ItemState::Pending if !task.waits_for.is_empty() => {
            parts.push(format!("waits for {}", task.waits_for.join(LIST_SEPARATOR)));
        }
        ItemState::Pending | ItemState::Cancelled => {}
    }
    match (task.state, paused) {
        (ItemState::Queued, true) => parts.push("waiting for agent launches to resume".to_owned()),
        (ItemState::Queued, false) => parts.push(format!(
            "waiting for agent slot ({}/{} in use)",
            slots.in_use, slots.total
        )),
        _ => {}
    }
    parts.extend(task.conflict.as_ref().map(conflict));
    parts.join(SEPARATOR)
}

fn attempt(task: &ItemStatus) -> Option<String> {
    task.attempt.map(|attempt| format!("attempt {attempt}"))
}

fn conflict(conflict: &Conflict) -> String {
    match conflict {
        Conflict::Merge { paths } => format!("merge: {}", paths.join(LIST_SEPARATOR)),
        Conflict::Checks { failed } => format!("checks: {}", failed.join(LIST_SEPARATOR)),
    }
}
