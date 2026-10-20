use surt_engine::project::StepLabel;
use surt_engine::runtime::RestartReason;

use super::{FIRST, label, named};
use crate::message::{Mark, Message};

pub(super) fn started(step: &StepLabel, attempt: u32, continued: bool) -> (Mark, Message) {
    if attempt > FIRST {
        return (
            Mark::Retry,
            Message::new(format!("{} attempt {attempt}", label(step))),
        );
    }
    if continued {
        return (
            Mark::Started,
            Message::new(format!(
                "{} continued (iteration {}, same conversation)",
                named(step),
                iteration(step)
            )),
        );
    }
    (
        Mark::Started,
        Message::new(format!("{} started", label(step))),
    )
}

pub(super) fn restarted(step: &StepLabel, reason: RestartReason) -> (Mark, Message) {
    (
        Mark::Retry,
        Message::new(format!(
            "{} restarted (iteration {}): {}",
            named(step),
            iteration(step),
            reason_name(reason)
        )),
    )
}

fn iteration(step: &StepLabel) -> u32 {
    step.iteration.unwrap_or(FIRST)
}

fn reason_name(reason: RestartReason) -> &'static str {
    match reason {
        RestartReason::TranscriptMissing => "transcript_missing",
        RestartReason::ContinuationRefused => "continuation_refused",
    }
}
