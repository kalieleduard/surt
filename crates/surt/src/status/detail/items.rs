use std::fmt::Write;

use surt_engine::project::{AgentSlots, Conflict, ItemState, ItemStatus, RunStatus};

use super::line;
use crate::driving::{local_time, short};
use crate::states::item_state_name;
use crate::symbols::{BLOCKED, ERROR, PAUSED, SEPARATOR, STARTED, STOPPED, SUCCESS, WAITING};

const LIST_SEPARATOR: &str = ", ";
const WORD_WIDTH: usize = 9;
const UNTITLED: &str = "items";

pub(super) fn render(text: &mut String, status: &RunStatus) {
    if status.items.is_empty() {
        return;
    }
    line(text, status.item_step.as_deref().unwrap_or(UNTITLED));
    let width = status
        .items
        .iter()
        .map(|item| item.key.chars().count())
        .max()
        .unwrap_or_default();
    let paused = status.paused.is_some();
    for item in &status.items {
        let (mark, word) = look(item);
        let entry = format!(
            "    {:<width$}  {mark} {word:<WORD_WIDTH$} {}",
            item.key,
            details(item, status.slots, paused)
        );
        let _ = writeln!(text, "{}", entry.trim_end());
    }
}

fn look(item: &ItemStatus) -> (&'static str, String) {
    let step = || item.step.clone().unwrap_or_default();
    let named = || item_state_name(item.state).to_owned();
    match item.state {
        ItemState::Succeeded => (SUCCESS, "merged".to_owned()),
        ItemState::Running => (STARTED, step()),
        ItemState::Queued => (WAITING, named()),
        ItemState::Pending => (PAUSED, named()),
        ItemState::Interrupted => (STOPPED, "stopped".to_owned()),
        ItemState::Blocked => (BLOCKED, named()),
        ItemState::Cancelled => (ERROR, named()),
    }
}

fn details(item: &ItemStatus, slots: AgentSlots, paused: bool) -> String {
    let mut parts = Vec::new();
    let iteration = item.iteration.map(|iteration| format!("iter {iteration}"));
    match item.state {
        ItemState::Succeeded => {
            parts.extend(iteration);
            parts.extend(
                item.merged_as
                    .as_deref()
                    .map(|sha| format!("as {}", short(sha))),
            );
        }
        ItemState::Running => {
            parts.extend(iteration);
            parts.extend(attempt(item));
            parts.extend(
                item.since
                    .and_then(local_time)
                    .map(|time| format!("since {time}")),
            );
        }
        ItemState::Queued | ItemState::Interrupted | ItemState::Blocked => {
            parts.extend(item.step.as_deref().map(|step| match &iteration {
                Some(iteration) => format!("{step} {iteration}"),
                None => step.to_owned(),
            }));
            parts.extend(attempt(item));
        }
        ItemState::Pending if !item.waits_for.is_empty() => {
            parts.push(format!("waits for {}", item.waits_for.join(LIST_SEPARATOR)));
        }
        ItemState::Pending | ItemState::Cancelled => {}
    }
    match (item.state, paused) {
        (ItemState::Queued, true) => parts.push("waiting for agent launches to resume".to_owned()),
        (ItemState::Queued, false) => parts.push(format!(
            "waiting for agent slot ({}/{} in use)",
            slots.in_use, slots.total
        )),
        _ => {}
    }
    parts.extend(item.conflict.as_ref().map(conflict));
    parts.join(SEPARATOR)
}

fn attempt(item: &ItemStatus) -> Option<String> {
    item.attempt.map(|attempt| format!("attempt {attempt}"))
}

fn conflict(conflict: &Conflict) -> String {
    match conflict {
        Conflict::Merge { paths } => format!("merge: {}", paths.join(LIST_SEPARATOR)),
        Conflict::Checks { failed } => format!("checks: {}", failed.join(LIST_SEPARATOR)),
    }
}
