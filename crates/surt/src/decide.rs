use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::{Decided, Project, RevisionOrigin};

use crate::continue_run::{active_run, continue_command};
use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::symbols::{DASH, ELLIPSIS};

const SHORT_SHA_CHARS: usize = 6;
const UNKNOWN_PRODUCER: &str = "the step that produced it";

#[derive(clap::Args)]
pub(crate) struct ApproveArgs {
    subject: String,
    #[arg(long, value_name = "N")]
    rev: Option<u32>,
}

#[derive(clap::Args)]
pub(crate) struct RejectArgs {
    subject: String,
    #[arg(short = 'm', long = "message", value_name = "reason", value_parser = NonEmptyStringValueParser::new())]
    reason: String,
    #[arg(long, value_name = "N")]
    rev: Option<u32>,
}

pub(crate) async fn approve(cwd: &Path, args: ApproveArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.subject).await?;
    let run = active_run(&subject)?;
    let decided = project
        .approve(run, args.rev, &project.actor().await?)
        .await?;
    warn_restored(&subject.name, &decided);
    approved(&subject.name, &decided).print_success();
    Ok(())
}

pub(crate) async fn reject(cwd: &Path, args: RejectArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.subject).await?;
    let run = active_run(&subject)?;
    let decided = project
        .reject(run, args.rev, &args.reason, &project.actor().await?)
        .await?;
    warn_restored(&subject.name, &decided);
    rejected(&subject.name, &decided).print_success();
    Ok(())
}

fn warn_restored(subject: &str, decided: &Decided) {
    for path in &decided.restored {
        Message::new(format!(
            ".surt/features/{subject}/{path} was missing; recreated from the latest revision"
        ))
        .print(Mark::Warning);
    }
}

fn approved(subject: &str, decided: &Decided) -> Message {
    let short = decided
        .sha256
        .get(..SHORT_SHA_CHARS)
        .unwrap_or(&decided.sha256);
    let origin = match decided.origin {
        RevisionOrigin::Agent => "agent",
        RevisionOrigin::Human => "human edit",
    };
    Message::new(format!(
        "approved {} rev {} ({origin}) sha256:{short}{ELLIPSIS} by {}",
        decided.artifact, decided.revision, decided.actor
    ))
    .way("next", continue_command(subject))
}

fn rejected(subject: &str, decided: &Decided) -> Message {
    let producer = decided.producer.as_deref().unwrap_or(UNKNOWN_PRODUCER);
    Message::new(format!(
        "rejected {} rev {} {DASH} {producer} will redo it reading your reason",
        decided.artifact, decided.revision
    ))
    .way("regenerate", continue_command(subject))
}
