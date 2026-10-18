mod detail;
mod overview;

use std::path::Path;

use surt_engine::project::{GateStatus, Project, RunState, RunStatus, StatusScope};

use crate::failure::Failure;
use crate::phase;
use crate::symbols::printable;

#[derive(clap::Args)]
pub(crate) struct StatusArgs {
    feature: Option<String>,
    #[arg(long, conflicts_with = "feature")]
    all: bool,
}

pub(crate) async fn run(cwd: &Path, args: StatusArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    if let Some(feature) = &args.feature {
        for status in project.status(StatusScope::Feature(feature)).await? {
            anstream::print!("{}", printable(&detail::render(&status)));
        }
        return Ok(());
    }
    let scope = if args.all {
        StatusScope::All
    } else {
        StatusScope::Active
    };
    let statuses = project.status(scope).await?;
    if statuses.is_empty() {
        anstream::eprintln!("no feature has an active run (surt status --all lists finished ones)");
        return Ok(());
    }
    anstream::print!("{}", printable(&overview::render(&statuses)));
    Ok(())
}

pub(crate) const STATE_SEPARATOR: &str = " + ";

pub(crate) fn state_name(state: RunState) -> &'static str {
    match state {
        RunState::Running => "RUNNING",
        RunState::AwaitingApproval => "AWAITING APPROVAL",
        RunState::Blocked => "BLOCKED",
        RunState::Interrupted => "INTERRUPTED",
        RunState::Succeeded => "SUCCEEDED",
        RunState::Cancelled => "CANCELLED",
    }
}

fn state_names(states: &[RunState]) -> String {
    states
        .iter()
        .map(|state| state_name(*state))
        .collect::<Vec<_>>()
        .join(STATE_SEPARATOR)
}

fn resume_command(status: &RunStatus) -> String {
    phase::current_command(&status.subject, |gate| {
        status
            .gates
            .iter()
            .any(|view| view.step == gate && matches!(view.status, GateStatus::Approved { .. }))
    })
}

#[cfg(test)]
mod tests;
