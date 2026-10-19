mod detail;
mod json;
mod overview;

use std::path::Path;

use surt_engine::project::{GateStatus, Project, RunSelector, RunState, RunStatus, StatusScope};

pub(crate) use detail::merge_into_base;

use crate::failure::Failure;
use crate::phase;
use crate::symbols::printable;

#[derive(clap::Args)]
pub(crate) struct StatusArgs {
    feature: Option<String>,
    #[arg(long, conflicts_with = "feature")]
    all: bool,
    #[arg(long, value_name = "N", requires = "feature")]
    run: Option<u32>,
    #[arg(long)]
    json: bool,
}

pub(crate) async fn run(cwd: &Path, args: StatusArgs) -> Result<(), Failure> {
    let project = Project::open_read_only(cwd).await?;
    if let Some(feature) = &args.feature {
        let statuses = project
            .status(StatusScope::Feature(feature, run_selector(args.run)))
            .await?;
        if args.json {
            return statuses
                .first()
                .map_or(Ok(()), |status| crate::json::print(json::feature(status)));
        }
        for status in statuses {
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
    if args.json {
        return crate::json::print(json::features(&statuses));
    }
    if statuses.is_empty() {
        anstream::eprintln!("no feature has an active run (surt status --all lists finished ones)");
        return Ok(());
    }
    anstream::print!("{}", printable(&overview::render(&statuses)));
    Ok(())
}

pub(crate) fn run_selector(run: Option<u32>) -> RunSelector {
    run.map_or(RunSelector::Current, RunSelector::Ordinal)
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

pub(crate) fn state_names(states: &[RunState]) -> String {
    states
        .iter()
        .map(|state| state_name(*state))
        .collect::<Vec<_>>()
        .join(STATE_SEPARATOR)
}

pub(crate) fn resume_command(status: &RunStatus) -> String {
    phase::current_command(&status.subject, |gate| {
        status
            .gates
            .iter()
            .any(|view| view.step == gate && matches!(view.status, GateStatus::Approved { .. }))
    })
}

#[cfg(test)]
mod tests;
