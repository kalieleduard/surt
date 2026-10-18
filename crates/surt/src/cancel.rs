mod confirmation;

use std::io::{self, IsTerminal};
use std::path::Path;

use surt_engine::project::{Cancellation, Project, RunId};
use tokio::sync::mpsc;

use crate::builtin;
use crate::driving::{print_progress, report_cleanup};
use crate::failure::Failure;
use crate::message::Message;
use crate::phase::active_run;
use crate::symbols::{DASH, printable};

#[derive(clap::Args)]
pub(crate) struct CancelArgs {
    feature: String,
    #[arg(short = 'y', long)]
    yes: bool,
}

pub(crate) async fn run(cwd: &Path, args: CancelArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let subject = project.resolve_subject(&args.feature).await?;
    let run = match (&subject.active_run, &subject.pending_cleanup) {
        (None, Some(run)) => run,
        _ => {
            let run = active_run(&subject)?;
            confirm(&project, &subject.name, run, args.yes).await?;
            run
        }
    };
    let (sink, events) = mpsc::unbounded_channel();
    let (cancellation, cleaned) = tokio::join!(project.cancel(run, sink), print_progress(events));
    ended(&subject.name, cancellation?).print_success();
    if let Some(cleanup) = cleaned {
        report_cleanup(&cleanup, format!("surt cancel {}", subject.name));
    }
    Ok(())
}

async fn confirm(project: &Project, subject: &str, run: &RunId, yes: bool) -> Result<(), Failure> {
    if yes {
        return Ok(());
    }
    if !io::stdin().is_terminal() {
        return Err(Failure::Refused(
            Message::new("cancel asks for a confirmation and there is no terminal to ask")
                .way("fix", format!("surt cancel {subject} --yes")),
        ));
    }
    let question = confirmation::question(subject, &project.cancel_impact(run).await?);
    if confirmation::ask(&printable(&question), io::stdin().lock(), io::stderr())? {
        return Ok(());
    }
    Err(Failure::Refused(Message::new(format!(
        "{subject} was not cancelled"
    ))))
}

fn ended(subject: &str, cancellation: Cancellation) -> Message {
    match cancellation {
        Cancellation::Cancelled => Message::new("run cancelled"),
        Cancellation::AlreadyCancelled => Message::new(format!("{subject} was already cancelled")),
        Cancellation::AlreadySucceeded => Message::new(format!(
            "{subject} had already succeeded {DASH} nothing to cancel"
        )),
    }
}

#[cfg(test)]
mod tests;
