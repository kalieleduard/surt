use std::fmt::Write;

use surt_engine::project::{AgentSlots, Conflict, TaskState, TaskStatus};

use super::line;
use crate::driving::{local_time, short};
use crate::phase::TASK_STEP;

const SEPARATOR: &str = " · ";
const LIST_SEPARATOR: &str = ", ";
const WORD_WIDTH: usize = 9;

pub(super) fn render(text: &mut String, tasks: &[TaskStatus], slots: AgentSlots) {
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
            details(task, slots)
        );
        let _ = writeln!(text, "{}", entry.trim_end());
    }
}

fn look(task: &TaskStatus) -> (&'static str, String) {
    let step = || task.step.clone().unwrap_or_default();
    match task.state {
        TaskState::Succeeded => ("✔", "merged".to_owned()),
        TaskState::Running => ("●", step()),
        TaskState::Queued => ("⏳", "queued".to_owned()),
        TaskState::Pending => ("⏸", "pending".to_owned()),
        TaskState::Interrupted => ("⏹", "stopped".to_owned()),
        TaskState::Blocked => ("⛔", "blocked".to_owned()),
        TaskState::Cancelled => ("✖", "cancelled".to_owned()),
    }
}

fn details(task: &TaskStatus, slots: AgentSlots) -> String {
    let mut parts = Vec::new();
    let iteration = task.iteration.map(|iteration| format!("iter {iteration}"));
    match task.state {
        TaskState::Succeeded => {
            parts.extend(iteration);
            parts.extend(
                task.merged_as
                    .as_deref()
                    .map(|sha| format!("as {}", short(sha))),
            );
        }
        TaskState::Running => {
            parts.extend(iteration);
            parts.extend(attempt(task));
            parts.extend(
                task.since
                    .and_then(local_time)
                    .map(|time| format!("since {time}")),
            );
        }
        TaskState::Queued | TaskState::Interrupted | TaskState::Blocked => {
            parts.extend(task.step.as_deref().map(|step| match &iteration {
                Some(iteration) => format!("{step} {iteration}"),
                None => step.to_owned(),
            }));
            parts.extend(attempt(task));
        }
        TaskState::Pending if !task.waits_for.is_empty() => {
            parts.push(format!("waits for {}", task.waits_for.join(LIST_SEPARATOR)));
        }
        TaskState::Pending | TaskState::Cancelled => {}
    }
    if task.state == TaskState::Queued {
        parts.push(format!(
            "waiting for agent slot ({}/{} in use)",
            slots.in_use, slots.total
        ));
    }
    parts.extend(task.conflict.as_ref().map(conflict));
    parts.join(SEPARATOR)
}

fn attempt(task: &TaskStatus) -> Option<String> {
    task.attempt.map(|attempt| format!("attempt {attempt}"))
}

fn conflict(conflict: &Conflict) -> String {
    match conflict {
        Conflict::Merge { paths } => format!("merge: {}", paths.join(LIST_SEPARATOR)),
        Conflict::Checks { failed } => format!("checks: {}", failed.join(LIST_SEPARATOR)),
    }
}
