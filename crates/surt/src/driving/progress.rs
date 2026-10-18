use surt_engine::project::{Cleanup, Progress, StepLabel};
use tokio::sync::mpsc::UnboundedReceiver;

use super::cleanup::summary;
use crate::message::{Mark, Message};

const FIRST: u32 = 1;
const SHORT_SHA_CHARS: usize = 7;

pub(super) async fn print(mut events: UnboundedReceiver<Progress>) -> Option<Cleanup> {
    let mut cleaned = None;
    while let Some(event) = events.recv().await {
        if let Progress::CleanedUp(cleanup) = event {
            cleaned = Some(cleanup);
            continue;
        }
        let (mark, message) = message(&event);
        message.print(mark);
    }
    cleaned
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

pub(super) fn short(sha: &str) -> &str {
    sha.get(..SHORT_SHA_CHARS).unwrap_or(sha)
}

fn subject(step: &StepLabel) -> String {
    match &step.item {
        Some(item) => format!("task {item}"),
        None => step.key.clone(),
    }
}

fn message(progress: &Progress) -> (Mark, Message) {
    match progress {
        Progress::StepStarted { step, attempt } if *attempt == FIRST => (
            Mark::Started,
            Message::new(format!("{} started", label(step))),
        ),
        Progress::StepStarted { step, attempt } => (
            Mark::Retry,
            Message::new(format!("{} attempt {attempt}", label(step))),
        ),
        Progress::StepFinished { step, outcome } => (
            Mark::Success,
            Message::new(format!("{} {outcome}", label(step))),
        ),
        Progress::Merged { step, merge } => (
            Mark::Success,
            Message::new(format!("{} merged ({})", subject(step), short(merge))),
        ),
        Progress::AttemptFailed {
            step,
            attempt,
            code,
            message,
        } => (
            Mark::Warning,
            Message::new(format!(
                "{} attempt {attempt} failed ({code}): {message}",
                label(step)
            )),
        ),
        Progress::AttemptInterrupted { step, attempt } => (
            Mark::Stopped,
            Message::new(format!("{} attempt {attempt} interrupted", label(step))),
        ),
        Progress::RevisionPublished { artifact, revision } => (
            Mark::Success,
            Message::new(format!("{artifact} rev {revision} published")),
        ),
        Progress::Salvaged { step, reference } => (
            Mark::Warning,
            Message::new(format!(
                "{} worktree had leftovers — saved to {reference}",
                label(step)
            ))
            .way("inspect", format!("git log -p {reference}")),
        ),
        Progress::WorktreeRepaired { step, path } => (
            Mark::Warning,
            Message::new(format!(
                "worktree for {} was missing — recreated at {}",
                label(step),
                path.display()
            )),
        ),
        Progress::EditRecorded { artifact, revision } => (
            Mark::Success,
            Message::new(format!(
                "your edit to {artifact} was recorded as rev {revision}"
            )),
        ),
        Progress::EditSuperseded { artifact, edit, by } => (
            Mark::Warning,
            Message::new(format!(
                "your edit to {artifact} was saved as rev {edit} and superseded by the agent's rev {by}"
            )),
        ),
        Progress::FileRestored {
            artifact,
            path,
            revision,
        } => (
            Mark::Warning,
            Message::new(format!(
                "{path} was missing; recreated from {artifact} rev {revision}"
            )),
        ),
        Progress::Recovering { driver } => (Mark::Warning, recovering(*driver)),
        Progress::OrphanStopped { step, attempt, pid } => (
            Mark::Warning,
            Message::new(format!(
                "{} attempt {attempt} (pid {pid}, alive) — stopped",
                label(step)
            )),
        ),
        Progress::Discarded { step, attempt } => (
            Mark::Error,
            Message::new(format!(
                "{} attempt {attempt} exited but was never recorded — discarded",
                label(step)
            )),
        ),
        Progress::IntegrationRecorded {
            step,
            branch,
            merge,
        } => (
            Mark::Success,
            Message::new(format!(
                "{} had completed ({branch} @ {}) — recorded",
                label(step),
                short(merge)
            )),
        ),
        Progress::CleanedUp(cleanup) => (Mark::Retry, Message::new(summary(cleanup))),
    }
}

fn recovering(driver: Option<u32>) -> Message {
    match driver {
        Some(pid) => Message::new(format!("previous driver (pid {pid}) is gone — recovering")),
        None => Message::new("previous driver is gone — recovering"),
    }
}

#[cfg(test)]
mod tests;
