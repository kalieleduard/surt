use surt_engine::project::{CleanupState, GateStatus, RunStatus, TaskState};

use super::state_names;
use crate::driving::label;
use crate::phase::Phase;
use crate::table;

const HEADERS: [&str; 5] = ["FEATURE", "RUN", "STATE", "DRIVER", "PROGRESS"];
const NONE: &str = "—";
const SEPARATOR: &str = " · ";
const COUNTED: [(TaskState, &str); 3] = [
    (TaskState::Running, "running"),
    (TaskState::Queued, "queued"),
    (TaskState::Blocked, "blocked"),
];

pub(super) fn render(statuses: &[RunStatus]) -> String {
    let mut rows = vec![HEADERS.map(str::to_owned)];
    rows.extend(statuses.iter().map(row));
    let mut text = table::render(&rows);
    if let Some(first) = statuses.first() {
        let in_use: u32 = statuses.iter().map(|status| status.slots.in_use).sum();
        text.push_str(&format!(
            "agent slots: {in_use}/{} in use\n",
            first.slots.total
        ));
    }
    text
}

fn row(status: &RunStatus) -> [String; 5] {
    [
        status.subject.clone(),
        status.run.to_string(),
        state_names(&status.states),
        status
            .driver
            .map_or_else(|| NONE.to_owned(), |driver| format!("pid {}", driver.pid)),
        progress(status),
    ]
}

fn progress(status: &RunStatus) -> String {
    let mut parts = Vec::new();
    if status.tasks.is_empty() {
        parts.extend(gate_progress(status));
    } else {
        parts.push(task_progress(status));
    }
    if parts.is_empty() {
        parts.extend(
            status
                .blocked
                .first()
                .map(|blocked| format!("{} blocked", label(&blocked.step))),
        );
    }
    if let Some(CleanupState::Pending { .. }) = status.cleanup {
        parts.push("cleanup pending".to_owned());
    }
    if parts.is_empty() {
        return NONE.to_owned();
    }
    parts.join(SEPARATOR)
}

fn task_progress(status: &RunStatus) -> String {
    let merged = count(status, TaskState::Succeeded);
    let mut parts = vec![format!("{merged}/{} merged", status.tasks.len())];
    for (state, word) in COUNTED {
        let found = count(status, state);
        if found > 0 {
            parts.push(format!("{found} {word}"));
        }
    }
    parts.join(SEPARATOR)
}

fn count(status: &RunStatus, state: TaskState) -> usize {
    status
        .tasks
        .iter()
        .filter(|task| task.state == state)
        .count()
}

fn gate_progress(status: &RunStatus) -> Option<String> {
    status.gates.iter().find_map(|gate| {
        let GateStatus::Waiting { revision } = gate.status else {
            return None;
        };
        let files = gate
            .revision
            .as_ref()
            .map_or(0, |summary| summary.files.len());
        let covers_items = Phase::of_gate(&gate.step).is_some_and(Phase::covers_item_files);
        Some(if covers_items {
            format!(
                "{} rev {revision} ({} tasks)",
                gate.artifact,
                files.saturating_sub(1)
            )
        } else {
            format!("{} rev {revision}", gate.artifact)
        })
    })
}
