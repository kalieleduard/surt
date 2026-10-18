mod activity;
mod screen;
mod wording;

use std::time::Duration;

use surt_engine::project::{Cleanup, Progress, StepLabel};
use tokio::sync::mpsc::UnboundedReceiver;

use screen::{Screen, now};
use wording::message;

const FIRST: u32 = 1;
const SHORT_SHA_CHARS: usize = 7;
const REDRAW: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(crate) struct Printed {
    pub(crate) cleaned: Option<Cleanup>,
    pub(crate) kept: bool,
    pub(crate) stop_requested: bool,
}

pub(crate) async fn print(mut events: UnboundedReceiver<Progress>) -> Printed {
    let mut screen = Screen::new(now());
    let mut redraw = tokio::time::interval(REDRAW);
    let mut printed = Printed::default();
    loop {
        tokio::select! {
            event = events.recv() => match event {
                None => break,
                Some(Progress::CleanedUp(cleanup)) => printed.cleaned = Some(cleanup),
                Some(Progress::RunKept) => printed.kept = true,
                Some(Progress::Activity(snapshot)) => screen.snapshot(snapshot, now()),
                Some(event) => {
                    printed.stop_requested |= event == Progress::StopRequested;
                    let (mark, message) = message(&event);
                    screen.line(mark, &message, now());
                }
            },
            _ = redraw.tick() => screen.tick(now()),
        }
    }
    screen.finish();
    printed
}

pub(crate) fn label(step: &StepLabel) -> String {
    let mut text = match &step.item {
        Some(item) => format!("task {item} {}", step.key),
        None => step.key.clone(),
    };
    if let Some(iteration) = step.iteration.filter(|iteration| *iteration > FIRST) {
        text.push_str(&format!(" iter {iteration}"));
    }
    text
}

pub(crate) fn short(sha: &str) -> &str {
    sha.get(..SHORT_SHA_CHARS).unwrap_or(sha)
}

#[cfg(test)]
mod tests;
