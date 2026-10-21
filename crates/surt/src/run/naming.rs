use jiff::Zoned;
use surt_engine::project::{FreeName, Project, Slug};

use super::RunArgs;
use crate::failure::Failure;
use crate::message::Message;
use crate::retry::{Retry, quoted};

const NAME_PLACEHOLDER: &str = "--name <slug>";

pub(super) async fn free_name(project: &Project, args: &RunArgs) -> Result<FreeName, Failure> {
    let today = Zoned::now().date();
    let Some(name) = &args.name else {
        return Ok(project.next_numbered_subject_name(today).await?);
    };
    let slug = Slug::derive(name).ok_or_else(|| Failure::Refused(no_slug(args, name)))?;
    Ok(project.free_subject_name(&slug, today).await?)
}

fn retry(args: &RunArgs) -> Retry {
    args.inputs.iter().fold(
        Retry::command(&["surt", "run", &args.workflow]),
        |retry, input| retry.word("--input").word(&input.quoted()),
    )
}

fn no_slug(args: &RunArgs, name: &str) -> Message {
    let headline = format!("cannot derive a name from --name {}", quoted(name));
    let fix = retry(args).word(NAME_PLACEHOLDER);
    Message::new(headline).way("fix", fix.render())
}
