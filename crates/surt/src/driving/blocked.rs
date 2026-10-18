use jiff::Timestamp;
use jiff::tz::TimeZone;
use surt_engine::project::{BlockReason, BlockedStep, LastFailure};

use super::progress::label;
use crate::message::{Mark, Message};
use crate::symbols::DASH;

pub(super) fn report(steps: &[BlockedStep], resume: String) -> Message {
    let mut messages: Vec<Message> = steps.iter().map(message).collect();
    let last = messages
        .pop()
        .unwrap_or_else(|| Message::new("nothing can run without you"));
    for message in &messages {
        message.print(Mark::Blocked);
    }
    last.way("resume", resume)
}

fn message(blocked: &BlockedStep) -> Message {
    let headline = format!(
        "{} blocked: {}",
        label(&blocked.step),
        reason(blocked.reason)
    );
    let Some(failure) = &blocked.last_failure else {
        return Message::new(headline);
    };
    Message::new(headline)
        .context(last_error(failure))
        .way("inspect", format!("surt inspect {}", failure.execution))
        .way("logs", format!("surt logs {}", failure.execution))
}

pub(crate) fn reason(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::AttemptsExhausted => "attempts exhausted",
        BlockReason::NotRetryable => "cannot be retried",
        BlockReason::IterationsExhausted => "iterations exhausted",
    }
}

pub(crate) fn last_error(failure: &LastFailure) -> String {
    let mut line = format!(
        "attempt {} failed ({}): {}",
        failure.attempt, failure.code, failure.message
    );
    if let Some(resets) = failure.resets_at.and_then(local_time) {
        line.push_str(&format!(" {DASH} resets {resets}"));
    }
    line
}

pub(crate) fn local_time(seconds: i64) -> Option<String> {
    let timestamp = Timestamp::from_second(seconds).ok()?;
    Some(
        timestamp
            .to_zoned(TimeZone::system())
            .strftime("%H:%M")
            .to_string(),
    )
}
