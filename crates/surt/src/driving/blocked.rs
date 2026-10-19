use jiff::Timestamp;
use jiff::tz::TimeZone;
use surt_engine::project::{BlockReason, BlockedStep, InterruptedMerge, LastFailure};

use super::progress::{label, short};
use crate::feature::word;
use crate::message::{Mark, Message};
use crate::symbols::DASH;

const INTERRUPTED_MERGE: &str = "merge_into_base_interrupted";
const RESUME: &str = "resume";
const THEN: &str = "then";

pub(super) fn report(
    steps: &[BlockedStep],
    interrupted: Option<&InterruptedMerge>,
    resume: String,
) -> Message {
    let mut messages: Vec<(Message, &str)> = steps
        .iter()
        .map(|blocked| message(blocked, interrupted))
        .collect();
    let (last, way) = messages
        .pop()
        .unwrap_or_else(|| (Message::new("nothing can run without you"), RESUME));
    for (message, _) in &messages {
        message.print(Mark::Blocked);
    }
    last.way(way, resume)
}

fn message(
    blocked: &BlockedStep,
    interrupted: Option<&InterruptedMerge>,
) -> (Message, &'static str) {
    match (&blocked.last_failure, interrupted) {
        (Some(failure), Some(merge)) if failure.code == INTERRUPTED_MERGE => {
            (interrupted_merge(merge), THEN)
        }
        _ => (step_message(blocked), RESUME),
    }
}

fn interrupted_merge(merge: &InterruptedMerge) -> Message {
    let path = merge.worktree.to_string_lossy();
    let worktree = word(&path);
    Message::new(format!(
        "merge into {} was interrupted; {path} may be partially updated",
        merge.branch
    ))
    .context(format!(
        "finish it:  git -C {worktree} merge --ff-only {}",
        short(&merge.to)
    ))
    .context(format!(
        "or undo it: git -C {worktree} reset --hard {}   (discards changes)",
        short(&merge.from)
    ))
}

fn step_message(blocked: &BlockedStep) -> Message {
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

#[cfg(test)]
mod tests;
