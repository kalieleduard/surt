mod gates;
mod tasks;

use std::fmt::Write;

use surt_engine::project::{CleanupState, LaunchPause, MergeIntoBase, RunState, RunStatus};

use super::{resume_command, state_names};
use crate::driving::{kept_line, label, last_error, local_time, reason, short};
use crate::symbols::{ARROW, BLOCKED, DASH, DOT, INTEGRATING, PAUSED, WARNING};

const LABEL_WIDTH: usize = 8;
const CONTINUATION: &str = "           ";

pub(super) fn render(status: &RunStatus) -> String {
    let resume = resume_command(status);
    let mut text = headline(status);
    if let Some(pause) = &status.paused {
        line(&mut text, &paused(pause));
    }
    if let Some(base) = &status.base {
        let reference = base.branch.as_deref().unwrap_or("HEAD");
        field(
            &mut text,
            "base",
            &format!("{reference} @ {}", short(&base.sha)),
        );
    }
    if let Some(branch) = &status.branch {
        field(
            &mut text,
            "branch",
            &format!(
                "{} @ {} {DOT} merge into base: {}",
                branch.name,
                short(&branch.tip),
                merge_into_base(status.merge_into_base)
            ),
        );
    }
    gates::render(&mut text, status);
    tasks::render(
        &mut text,
        &status.items,
        status.slots,
        status.paused.is_some(),
    );
    blocked(&mut text, status, &resume);
    for unattended in &status.unattended {
        line(
            &mut text,
            &format!(
                "{WARNING} {} still running unattended (pid {}) {DASH} stopped on next: {resume}",
                label(&unattended.step),
                unattended.pid
            ),
        );
    }
    for pending in &status.integrating {
        line(
            &mut text,
            &format!(
                "{INTEGRATING} {}   {}: {} {ARROW} {} (pending)",
                label(&pending.step),
                pending.branch,
                short(&pending.from),
                short(&pending.to)
            ),
        );
    }
    cleanup(&mut text, status, &resume);
    text
}

pub(crate) fn merge_into_base(mode: MergeIntoBase) -> &'static str {
    match mode {
        MergeIntoBase::Never => "never",
        MergeIntoBase::FfOnly => "ff-only",
    }
}

fn headline(status: &RunStatus) -> String {
    let driver = match (status.driver, status.states.as_slice()) {
        (Some(driver), _) => {
            let since = driver
                .since
                .and_then(local_time)
                .map(|time| format!(" since {time}"))
                .unwrap_or_default();
            format!("   driver pid {}{since}", driver.pid)
        }
        (None, [RunState::Succeeded | RunState::Cancelled]) => String::new(),
        (None, _) => "   (no driver)".to_owned(),
    };
    let of = match status.runs {
        0 | 1 => String::new(),
        runs => format!(" of {runs}"),
    };
    format!(
        "{} {DOT} run {}{of}   {}{driver}\n",
        status.subject,
        status.run,
        state_names(&status.states)
    )
}

fn paused(pause: &LaunchPause) -> String {
    let until = match pause.resets_at.and_then(local_time) {
        Some(time) => format!("resets {time}"),
        None => "retrying with backoff".to_owned(),
    };
    format!(
        "{PAUSED} agent launches paused: {} {DASH} {until}",
        pause.code
    )
}

fn blocked(text: &mut String, status: &RunStatus, resume: &str) {
    for step in &status.blocked {
        line(
            text,
            &format!(
                "{BLOCKED} {} blocked: {}",
                label(&step.step),
                reason(step.reason)
            ),
        );
        let Some(failure) = &step.last_failure else {
            continue;
        };
        line(text, &format!("   {}", last_error(failure)));
        line(
            text,
            &format!("   inspect: surt inspect {}", failure.execution),
        );
        line(text, &format!("   logs: surt logs {}", failure.execution));
    }
    if !status.blocked.is_empty() {
        line(text, &format!("resume: {resume}"));
    }
}

fn cleanup(text: &mut String, status: &RunStatus, resume: &str) {
    let Some(cleanup) = &status.cleanup else {
        return;
    };
    let (headline, kept, left) = match cleanup {
        CleanupState::Finished { kept } => ("done", kept, &Vec::new()),
        CleanupState::Pending { left, kept } => ("pending", kept, left),
        CleanupState::Kept => {
            let kept = format!(
                "kept (on_success = \"keep\") {DASH} remove with: surt delete {}",
                status.subject
            );
            return field(text, "cleanup", &kept);
        }
    };
    field(text, "cleanup", headline);
    for left in left {
        let _ = writeln!(
            text,
            "{CONTINUATION}could not remove {} ({})",
            left.item, left.reason
        );
    }
    for kept in kept {
        let _ = writeln!(text, "{CONTINUATION}{}", kept_line(kept));
    }
    if matches!(cleanup, CleanupState::Pending { .. }) {
        let _ = writeln!(text, "{CONTINUATION}retry: {resume}");
    }
}

fn field(text: &mut String, name: &str, value: &str) {
    let _ = writeln!(text, "  {name:<LABEL_WIDTH$} {value}");
}

fn line(text: &mut String, value: &str) {
    let _ = writeln!(text, "  {value}");
}
