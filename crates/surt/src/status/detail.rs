mod gates;
mod tasks;

use std::fmt::Write;

use surt_engine::project::{CleanupState, RunState, RunStatus};

use super::{resume_command, state_names};
use crate::driving::{kept_line, label, last_error, local_time, reason, short};

const LABEL_WIDTH: usize = 8;
const CONTINUATION: &str = "           ";

pub(super) fn render(status: &RunStatus) -> String {
    let resume = resume_command(status);
    let mut text = headline(status);
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
            &format!("{} @ {}", branch.name, short(&branch.tip)),
        );
    }
    gates::render(&mut text, status);
    tasks::render(&mut text, &status.tasks);
    blocked(&mut text, status, &resume);
    for unattended in &status.unattended {
        line(
            &mut text,
            &format!(
                "⚠ {} still running unattended (pid {}) — stopped on next: {resume}",
                label(&unattended.step),
                unattended.pid
            ),
        );
    }
    for pending in &status.integrating {
        line(
            &mut text,
            &format!(
                "⟳ {}   {}: {} → {} (pending)",
                label(&pending.step),
                pending.branch,
                short(&pending.from),
                short(&pending.to)
            ),
        );
    }
    cleanup(&mut text, status.cleanup.as_ref(), &resume);
    text
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
    format!(
        "{} · run {}   {}{driver}\n",
        status.subject,
        status.run,
        state_names(&status.states)
    )
}

fn blocked(text: &mut String, status: &RunStatus, resume: &str) {
    for step in &status.blocked {
        line(
            text,
            &format!("⛔ {} blocked: {}", label(&step.step), reason(step.reason)),
        );
        let Some(failure) = &step.last_failure else {
            continue;
        };
        line(text, &format!("   {}", last_error(failure)));
        if let Some(log) = &failure.log {
            line(text, &format!("   inspect: {}", log.display()));
        }
    }
    if !status.blocked.is_empty() {
        line(text, &format!("resume: {resume}"));
    }
}

fn cleanup(text: &mut String, cleanup: Option<&CleanupState>, resume: &str) {
    let Some(cleanup) = cleanup else {
        return;
    };
    let (headline, kept, left) = match cleanup {
        CleanupState::Finished { kept } => ("done", kept, &Vec::new()),
        CleanupState::Pending { left, kept } => ("pending", kept, left),
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
