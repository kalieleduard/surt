use surt_engine::project::{Progress, StepLabel};

use super::super::blocked::local_time;
use super::super::cleanup::summary;
use super::activity::duration;
use super::{FIRST, label, short};
use crate::message::{Mark, Message};
use crate::refusal::config_location;
use crate::symbols::{DASH, ELLIPSIS, SEPARATOR};

fn subject(step: &StepLabel) -> String {
    match &step.item {
        Some(item) => format!("task {item}"),
        None => step.key.clone(),
    }
}

pub(super) fn message(progress: &Progress) -> (Mark, Message) {
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
                "{} worktree had leftovers {DASH} saved to {reference}",
                label(step)
            ))
            .way("inspect", format!("git log -p {reference}")),
        ),
        Progress::WorktreeRepaired { step, path } => (
            Mark::Warning,
            Message::new(format!(
                "worktree for {} was missing {DASH} recreated at {}",
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
                "{} attempt {attempt} (pid {pid}, alive) {DASH} stopped",
                label(step)
            )),
        ),
        Progress::Discarded { step, attempt } => (
            Mark::Error,
            Message::new(format!(
                "{} attempt {attempt} exited but was never recorded {DASH} discarded",
                label(step)
            )),
        ),
        Progress::InterruptedWithDriver { step, attempt } => (
            Mark::Warning,
            Message::new(format!(
                "{} attempt {attempt} was interrupted with the previous driver {DASH} discarded",
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
                "{} had completed ({branch} @ {}) {DASH} recorded",
                label(step),
                short(merge)
            )),
        ),
        Progress::CancelRequested { driver } => (Mark::Waiting, cancel_requested(*driver)),
        Progress::StopRequested => (
            Mark::Waiting,
            Message::new(format!(
                "stop requested from another terminal; stopping{ELLIPSIS}"
            )),
        ),
        Progress::ConfigInvalid {
            path,
            line,
            reason,
            limit,
        } => {
            let location = config_location(path, *line);
            (
                Mark::Warning,
                Message::new(format!(
                    "invalid configuration in {location} {DASH} keeping max_concurrent_agents = {limit}"
                ))
                .context(reason)
                .way("fix", format!("edit {location}")),
            )
        }
        Progress::Paused {
            until,
            waiting,
            backoff,
        } => (Mark::Paused, paused(*until, *waiting, *backoff)),
        Progress::Resumed { paused_seconds } => (
            Mark::Retry,
            Message::new(format!(
                "agent launches resumed after {}",
                duration(i64::try_from(*paused_seconds).unwrap_or(i64::MAX))
            )),
        ),
        Progress::CleanedUp(cleanup) => (Mark::Retry, Message::new(summary(cleanup))),
        Progress::Activity(snapshot) => (
            Mark::Waiting,
            Message::new(format!("{} running", snapshot.running.len())),
        ),
    }
}

fn cancel_requested(driver: Option<u32>) -> Message {
    match driver {
        Some(pid) => Message::new(format!(
            "cancel requested; driver pid {pid} is stopping it{ELLIPSIS}"
        )),
        None => Message::new(format!(
            "cancel requested; waiting for the driver to stop{ELLIPSIS}"
        )),
    }
}

fn paused(until: i64, waiting: usize, backoff: Option<u64>) -> Message {
    let at = local_time(until).unwrap_or_else(|| until.to_string());
    let delay = backoff
        .map(|seconds| {
            let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
            format!(" (backoff {})", duration(seconds))
        })
        .unwrap_or_default();
    Message::new(format!(
        "agent launches paused until {at}{delay} {DASH} {waiting} waiting{SEPARATOR}Ctrl-C to stop (resumable)"
    ))
}

fn recovering(driver: Option<u32>) -> Message {
    match driver {
        Some(pid) => Message::new(format!(
            "previous driver (pid {pid}) is gone {DASH} recovering"
        )),
        None => Message::new(format!("previous driver is gone {DASH} recovering")),
    }
}
