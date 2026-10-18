mod question;

use std::io::{self, IsTerminal};
use std::path::Path;

use surt_engine::project::{Deletion, Left, Project};

use crate::builtin;
use crate::cancel::ask;
use crate::failure::Failure;
use crate::message::Message;
use crate::symbols::{DASH, printable};

#[derive(clap::Args)]
pub(crate) struct DeleteArgs {
    feature: String,
    #[arg(short = 'y', long)]
    yes: bool,
}

pub(crate) async fn run(cwd: &Path, args: DeleteArgs) -> Result<(), Failure> {
    let project = Project::open(cwd, builtin::catalog()).await?;
    let subject = project.resolve_subject(&args.feature).await?.name;
    if !args.yes {
        confirm(&project, &subject).await?;
    }
    match project.delete(&subject).await? {
        Deletion::Deleted => {
            Message::new(format!("{subject} deleted")).print_success();
            Ok(())
        }
        Deletion::Incomplete { left } => Err(Failure::Incomplete(incomplete(&subject, &left))),
    }
}

async fn confirm(project: &Project, subject: &str) -> Result<(), Failure> {
    let impact = project.delete_impact(subject).await?;
    if !io::stdin().is_terminal() {
        return Err(Failure::Refused(
            Message::new("delete asks for a confirmation and there is no terminal to ask")
                .way("fix", format!("surt delete {subject} --yes")),
        ));
    }
    let question = question::question(&impact);
    if ask(&printable(&question), io::stdin().lock(), io::stderr())? {
        return Ok(());
    }
    Err(Failure::Refused(Message::new(format!(
        "{subject} was not deleted"
    ))))
}

fn incomplete(subject: &str, left: &[Left]) -> Message {
    let mut lines = left
        .iter()
        .map(|left| format!("could not remove {} ({})", left.item, left.reason));
    let first = lines.next().unwrap_or_default();
    lines
        .fold(
            Message::new(format!("delete incomplete {DASH} {first}")),
            Message::context,
        )
        .way("retry", format!("surt delete {subject}"))
}

#[cfg(test)]
mod tests;
