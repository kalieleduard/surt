use surt_engine::project::{ActivityPause, ActivitySnapshot, RunningStep};

use super::row::Row;
use super::spinner;
use crate::driving::local_time;
use crate::duration::rounded;
use crate::symbols::{DASH, ELLIPSIS, PAUSED, SEPARATOR, Symbols};

const QUIET_LIMIT: i64 = 5 * 60;
const MILLIS_PER_SECOND: i64 = 1000;
const FIRST: u32 = 1;
const SHOWN_STEPS: usize = 6;
const GUTTER: &str = "  ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Terminal {
    Tty { width: usize, symbols: Symbols },
    Plain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Output {
    Footer(Vec<Row>),
    Line(String),
}

pub(super) struct Activity {
    pub(super) snapshot: Option<ActivitySnapshot>,
    pub(super) started: i64,
    pub(super) last_line: i64,
}

pub(super) fn on_tick(activity: &Activity, now_ms: i64, terminal: Terminal) -> Option<Output> {
    let seconds = now_ms.div_euclid(MILLIS_PER_SECOND);
    let snapshot = activity.snapshot.as_ref();
    match terminal {
        Terminal::Tty { width, symbols } => {
            let frame = spinner::frame(now_ms, symbols);
            let rows = match snapshot {
                Some(snapshot) => block(snapshot, activity.started, seconds, frame),
                None => vec![Row {
                    lead: format!("{frame} "),
                    label: String::new(),
                    faded: rounded(seconds - activity.started),
                }],
            };
            let rows = rows.into_iter().map(|row| row.rendered(symbols).fit(width));
            Some(Output::Footer(rows.collect()))
        }
        Terminal::Plain => still_running(snapshot?, activity.last_line, seconds).map(Output::Line),
    }
}

fn block(snapshot: &ActivitySnapshot, started: i64, now: i64, frame: &str) -> Vec<Row> {
    let shown = &snapshot.running[..snapshot.running.len().min(SHOWN_STEPS)];
    let names: Vec<String> = shown.iter().map(name).collect();
    let column = names.iter().map(|name| name.chars().count()).max();
    let mut rows: Vec<Row> = shown
        .iter()
        .zip(names)
        .map(|(step, name)| Row {
            lead: format!("{frame} "),
            label: format!("{name:<width$}{GUTTER}", width = column.unwrap_or(0)),
            faded: rounded(now - step.started_at),
        })
        .collect();
    let lead = match rows.is_empty() {
        true => format!("{frame} "),
        false => " ".repeat(frame.chars().count() + 1),
    };
    rows.push(Row {
        lead,
        label: String::new(),
        faded: summary(snapshot, shown.len(), started, now),
    });
    rows
}

fn summary(snapshot: &ActivitySnapshot, shown: usize, started: i64, now: i64) -> String {
    let mut parts = Vec::new();
    let hidden = snapshot.running.len() - shown;
    if hidden > 0 {
        parts.push(format!("+{hidden} more running"));
    }
    match snapshot.paused {
        Some(pause) => {
            parts.push(format!("{PAUSED} {}", paused_until(pause)));
            parts.push(format!("{} waiting", snapshot.queued));
        }
        None if snapshot.queued > 0 => parts.push(format!("{} queued", snapshot.queued)),
        None => {}
    }
    parts.push(format!(
        "slots {}/{}",
        snapshot.slots_in_use, snapshot.slots_limit
    ));
    parts.push(rounded(now - started));
    parts.join(SEPARATOR)
}

fn still_running(snapshot: &ActivitySnapshot, last_line: i64, now: i64) -> Option<String> {
    if now - last_line < QUIET_LIMIT {
        return None;
    }
    let mut parts = Vec::new();
    if !snapshot.running.is_empty() {
        let running: Vec<String> = snapshot
            .running
            .iter()
            .map(|step| format!("{} ({})", name(step), rounded(now - step.started_at)))
            .collect();
        parts.push(format!("still running: {}", running.join(", ")));
    }
    if let Some(pause) = snapshot.paused {
        parts.push(format!(
            "agent launches {} {DASH} {} waiting",
            paused_until(pause),
            snapshot.queued
        ));
    }
    if parts.is_empty() {
        return None;
    }
    Some(format!("{ELLIPSIS} {}", parts.join(SEPARATOR)))
}

pub(super) fn paused_until(pause: ActivityPause) -> String {
    let at = local_time(pause.until).unwrap_or_else(|| pause.until.to_string());
    let delay = pause
        .backoff
        .map(|seconds| format!(" (backoff {})", rounded(seconds)))
        .unwrap_or_default();
    format!("paused until {at}{delay}")
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

#[cfg(test)]
mod tests;
