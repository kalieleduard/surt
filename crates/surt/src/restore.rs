use std::path::Path;

use surt_engine::project::{Change, Project, RestoreImpact};

use crate::confirmation::confirmed;
use crate::failure::Failure;
use crate::message::Message;
use crate::symbols::MINUS;
use crate::table;

#[derive(clap::Args)]
pub(crate) struct RestoreArgs {
    feature: String,
    artifact: String,
    #[arg(short = 'y', long)]
    yes: bool,
}

pub(crate) async fn run(cwd: &Path, args: RestoreArgs) -> Result<(), Failure> {
    let project = Project::open(cwd).await?;
    let subject = project.resolve_subject(&args.feature).await?.name;
    let impact = project.restore_impact(&subject, &args.artifact).await?;
    if impact.changes.is_empty() {
        unchanged(&impact).print_success();
        return Ok(());
    }
    if !args.yes {
        confirm(&subject, &impact)?;
    }
    let done = project.restore(&subject, &args.artifact).await?;
    match done.changes.as_slice() {
        [] => unchanged(&done),
        _ => restored(&done),
    }
    .print_success();
    Ok(())
}

fn confirm(subject: &str, impact: &RestoreImpact) -> Result<(), Failure> {
    let retry = format!("surt restore {subject} {} --yes", impact.artifact);
    if confirmed("restore", retry, &question(subject, impact))? {
        return Ok(());
    }
    Err(Failure::Refused(Message::new(format!(
        "{} was not restored",
        impact.artifact
    ))))
}

fn question(subject: &str, impact: &RestoreImpact) -> String {
    let rows: Vec<[String; 2]> = impact
        .changes
        .iter()
        .map(|file| [file.path.clone(), described(file.change)])
        .collect();
    let listed: String = table::render(&rows)
        .lines()
        .map(|line| format!("  {line}\n"))
        .collect();
    format!(
        "discard your unrecorded edit to the {} of {subject}?\n{listed}",
        impact.artifact
    )
}

fn described(change: Change) -> String {
    match change {
        Change::Changed { added, removed } => format!("+{added} {MINUS}{removed}"),
        Change::New => "new file".to_owned(),
        Change::Missing => "missing".to_owned(),
    }
}

fn restored(impact: &RestoreImpact) -> Message {
    Message::new(format!(
        "restored {} rev {}; your edit was discarded",
        impact.artifact, impact.revision
    ))
}

fn unchanged(impact: &RestoreImpact) -> Message {
    Message::new(format!(
        "{} already matches rev {}; nothing to restore",
        impact.artifact, impact.revision
    ))
}

#[cfg(test)]
mod tests;
