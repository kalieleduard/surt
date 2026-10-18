use surt_engine::project::{ActivitySnapshot, RunningStep};

use crate::symbols::{ELLIPSIS, HORIZONTAL, SEPARATOR};

const QUIET_LIMIT: i64 = 5 * 60;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
const FIRST: u32 = 1;
const RULE_WIDTH: usize = 2;
const LAST_COLUMN: usize = 1;
const ELLIPSIS_ROOM: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Terminal {
    Tty { width: usize },
    Plain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Output {
    Footer(String),
    Line(String),
}

pub(super) struct Activity {
    pub(super) snapshot: Option<ActivitySnapshot>,
    pub(super) started: i64,
    pub(super) last_line: i64,
}

pub(super) fn on_tick(activity: &Activity, now: i64, terminal: Terminal) -> Option<Output> {
    let snapshot = activity.snapshot.as_ref()?;
    match terminal {
        Terminal::Tty { width } => Some(Output::Footer(fit(
            &footer(snapshot, activity.started, now),
            width,
        ))),
        Terminal::Plain => still_running(snapshot, activity.last_line, now).map(Output::Line),
    }
}

fn footer(snapshot: &ActivitySnapshot, started: i64, now: i64) -> String {
    let mut parts = vec![format!("{} running", snapshot.running.len())];
    if !snapshot.running.is_empty() {
        let running: Vec<String> = snapshot
            .running
            .iter()
            .map(|step| format!("{} {}", name(step), duration(now - step.started_at)))
            .collect();
        parts[0].push_str(&format!(" ({})", running.join(", ")));
    }
    if snapshot.queued > 0 {
        parts.push(format!("{} queued", snapshot.queued));
    }
    parts.push(format!(
        "slots {}/{}",
        snapshot.slots_in_use, snapshot.slots_limit
    ));
    parts.push(duration(now - started));
    let rule = HORIZONTAL.repeat(RULE_WIDTH);
    format!("{rule} {} {rule}", parts.join(SEPARATOR))
}

fn fit(text: &str, width: usize) -> String {
    let limit = width.saturating_sub(LAST_COLUMN);
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let kept: String = text
        .chars()
        .take(limit.saturating_sub(ELLIPSIS_ROOM))
        .collect();
    format!("{}{ELLIPSIS}", kept.trim_end())
}

fn still_running(snapshot: &ActivitySnapshot, last_line: i64, now: i64) -> Option<String> {
    if snapshot.running.is_empty() || now - last_line < QUIET_LIMIT {
        return None;
    }
    let running: Vec<String> = snapshot
        .running
        .iter()
        .map(|step| format!("{} ({})", name(step), duration(now - step.started_at)))
        .collect();
    Some(format!("{ELLIPSIS} still running: {}", running.join(", ")))
}

fn name(running: &RunningStep) -> String {
    let step = &running.step;
    let mut text = match &step.item {
        Some(item) => format!("{item} {}", step.key),
        None => step.key.clone(),
    };
    if let Some(iteration) = step.iteration.filter(|iteration| *iteration > FIRST) {
        text.push_str(&format!(" iter {iteration}"));
    }
    text
}

fn duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    if seconds < SECONDS_PER_MINUTE {
        format!("{seconds}s")
    } else if seconds < SECONDS_PER_HOUR {
        format!("{}m", seconds / SECONDS_PER_MINUTE)
    } else {
        format!(
            "{}h{:02}m",
            seconds / SECONDS_PER_HOUR,
            seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE
        )
    }
}

#[cfg(test)]
mod tests;
