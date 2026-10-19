use std::path::Path;

use clap::builder::NonEmptyStringValueParser;
use surt_engine::project::{Decided, Project, RevisionOrigin};

use crate::failure::Failure;
use crate::message::{Mark, Message};
use crate::phase::{Phase, active_run};
use crate::symbols::{DASH, ELLIPSIS};

const SHORT_SHA_CHARS: usize = 6;
const ITEM_FILE_EXTENSION: &str = ".md";

#[derive(clap::Args)]
pub(crate) struct ApproveArgs {
    feature: String,
    #[arg(long, value_name = "N")]
    rev: Option<u32>,
}

#[derive(clap::Args)]
pub(crate) struct RejectArgs {
    feature: String,
    #[arg(short = 'm', long = "message", value_name = "reason", value_parser = NonEmptyStringValueParser::new())]
    reason: String,
    #[arg(long, value_name = "N")]
    rev: Option<u32>,
}

pub(crate) async fn approve(cwd: &Path, args: ApproveArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.feature).await?;
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
    let subject = project.resolve_subject(&args.feature).await?;
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
    let message = Message::new(format!(
        "approved {} rev {} ({origin}) sha256:{short}{ELLIPSIS} by {}",
        decided.artifact, decided.revision, decided.actor
    ));
    match Phase::of_gate(&decided.gate) {
        Some(phase) => message.way("next", phase.next_command(subject)),
        None => message.way("inspect", format!("surt status {subject}")),
    }
}

fn rejected(subject: &str, decided: &Decided) -> Message {
    let what = format!("rejected {} rev {}", decided.artifact, decided.revision);
    let Some(phase) = Phase::of_gate(&decided.gate) else {
        return Message::new(what).way("inspect", format!("surt status {subject}"));
    };
    let item_files: Vec<&str> = decided
        .files
        .iter()
        .map(String::as_str)
        .filter(|path| phase.covers_item_files() && path.ends_with(ITEM_FILE_EXTENSION))
        .collect();
    let headline = match item_files.as_slice() {
        [] => format!(
            "{what} {DASH} the {} will redo it reading your reason",
            phase.agent()
        ),
        [only] => format!("{what} (with {only})"),
        [first, .., last] => format!("{what} (with {first} {ELLIPSIS} {last})"),
    };
    Message::new(headline).way("regenerate", phase.resume_command(subject))
}
