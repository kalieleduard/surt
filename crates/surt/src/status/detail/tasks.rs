use std::fmt::Write;

use surt_engine::project::{TaskState, TaskStatus};

use super::line;
use crate::driving::{local_time, short};

const SEPARATOR: &str = " · ";
const WORD_WIDTH: usize = 9;

pub(super) fn render(text: &mut String, tasks: &[TaskStatus]) {
    if tasks.is_empty() {
        return;
    }
    line(text, "tasks");
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
            details(task)
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

fn details(task: &TaskStatus) -> String {
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
            let step = task.step.clone().unwrap_or_default();
            parts.push(match iteration {
                Some(iteration) => format!("{step} {iteration}"),
                None => step,
            });
            parts.extend(attempt(task));
        }
        TaskState::Pending | TaskState::Cancelled => {}
    }
    parts.join(SEPARATOR)
}

fn attempt(task: &TaskStatus) -> Option<String> {
    task.attempt.map(|attempt| format!("attempt {attempt}"))
}
