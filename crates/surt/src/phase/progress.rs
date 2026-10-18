use surt_engine::project::{Progress, StepLabel};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::message::{Mark, Message};

const FIRST: u32 = 1;

pub(super) async fn print(mut events: UnboundedReceiver<Progress>) {
    while let Some(event) = events.recv().await {
        let (mark, line) = line(&event);
        Message::new(line).print(mark);
    }
}

pub(super) fn label(step: &StepLabel) -> String {
    let mut text = match &step.item {
        Some(item) => format!("task {item} {}", step.key),
        None => step.key.clone(),
    };
    if let Some(iteration) = step.iteration.filter(|iteration| *iteration > FIRST) {
        text.push_str(&format!(" iter {iteration}"));
    }
    text
}

fn line(progress: &Progress) -> (Mark, String) {
    match progress {
        Progress::StepStarted { step, attempt } if *attempt == FIRST => {
            (Mark::Started, format!("{} started", label(step)))
        }
        Progress::StepStarted { step, attempt } => {
            (Mark::Retry, format!("{} attempt {attempt}", label(step)))
        }
        Progress::StepFinished { step, outcome } => {
            (Mark::Success, format!("{} {outcome}", label(step)))
        }
        Progress::AttemptFailed {
            step,
            attempt,
            code,
            message,
        } => (
            Mark::Warning,
            format!(
                "{} attempt {attempt} failed ({code}): {message}",
                label(step)
            ),
        ),
        Progress::AttemptInterrupted { step, attempt } => (
            Mark::Stopped,
            format!("{} attempt {attempt} interrupted", label(step)),
        ),
        Progress::RevisionPublished { artifact, revision } => (
            Mark::Success,
            format!("{artifact} rev {revision} published"),
        ),
    }
}
